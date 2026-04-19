use core::sync::atomic::Ordering;
use crate::task::telemetry::{GLOBAL_TELEMETRY, log_heartbeat};
use crate::arch::x86_64::cpu::{self, SimdState};
use crate::arch::x86_64::interrupts;
use super::{Task, TaskId};
use alloc::{collections::{BTreeMap, BTreeSet}, sync::Arc};
use core::hint::spin_loop;
use core::task::{Waker, Context, Poll};
use crossbeam_queue::SegQueue;
use spin::Mutex;

pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<SegQueue<TaskId>>,
    queued_tasks: Arc<Mutex<BTreeSet<TaskId>>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(SegQueue::new()),
            queued_tasks: Arc::new(Mutex::new(BTreeSet::new())),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        if self.tasks.insert(task_id, task).is_some() {
            return;
        }

        GLOBAL_TELEMETRY.tasks_spawned.fetch_add(1, Ordering::Relaxed);
        enqueue_task(&self.task_queue, &self.queued_tasks, task_id);
    }

    fn run_ready_tasks(&mut self) {
        let Self {
            tasks,
            task_queue,
            queued_tasks,
            waker_cache,
        } = self;

        while let Some(task_id) = task_queue.pop() {
            dequeue_task(queued_tasks, task_id);

            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue,
            };
            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::into_waker(task_id, task_queue.clone(), queued_tasks.clone()));
            let mut context = Context::from_waker(waker);
            // TUFF-RADICAL: Surgical SIMD Context Guard
            let simd_enabled = cpu::detect_features().simd_enabled;
            if simd_enabled {
                if task.simd_state.is_none() {
                    task.simd_state = Some(alloc::boxed::Box::new(SimdState::new()));
                }
                unsafe { task.simd_state.as_ref().unwrap().restore(); }
            }

            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                    dequeue_task(queued_tasks, task_id);
                }
                Poll::Pending => {
                    if simd_enabled {
                        unsafe { task.simd_state.as_mut().unwrap().save(); }
                    }
                }
            }
        }
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            log_heartbeat(interrupts::current_tick());
            self.sleep_if_idle();
        }
    }

    fn sleep_if_idle(&self) {
        if self.task_queue.is_empty() {
            if interrupts::interrupt_timer_ready() {
                x86_64::instructions::interrupts::enable_and_hlt();
                x86_64::instructions::interrupts::disable();
            } else {
                interrupts::advance_cooperative_tick();
                spin_loop();
            }
        }
    }
}

use alloc::task::Wake;

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<SegQueue<TaskId>>,
    queued_tasks: Arc<Mutex<BTreeSet<TaskId>>>,
}

impl TaskWaker {
    fn into_waker(
        task_id: TaskId,
        task_queue: Arc<SegQueue<TaskId>>,
        queued_tasks: Arc<Mutex<BTreeSet<TaskId>>>,
    ) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
            queued_tasks,
        }))
    }

    fn wake_task(&self) {
        enqueue_task(&self.task_queue, &self.queued_tasks, self.task_id);
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}

fn enqueue_task(
    task_queue: &Arc<SegQueue<TaskId>>,
    queued_tasks: &Arc<Mutex<BTreeSet<TaskId>>>,
    task_id: TaskId,
) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut queued = queued_tasks.lock();
        if queued.insert(task_id) {
            task_queue.push(task_id);
        }
    });
}

fn dequeue_task(queued_tasks: &Arc<Mutex<BTreeSet<TaskId>>>, task_id: TaskId) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        queued_tasks.lock().remove(&task_id);
    });
}
