import pty
import os
import sys
import time

def run_ssh_sudo_command(user, host, password, command):
    # ssh + sudo command with password handling for both
    # We use -S for sudo to read from stdin
    full_command = f"sudo -S {command}"
    cmd = ['ssh', '-o', 'StrictHostKeyChecking=no', f'{user}@{host}', full_command]
    
    pid, fd = pty.fork()
    
    if pid == 0:
        os.execvp('ssh', cmd)
    else:
        output = b""
        ssh_pass_sent = False
        sudo_pass_sent = False
        
        start_time = time.time()
        while time.time() - start_time < 30: # 30s timeout
            try:
                data = os.read(fd, 1024)
                if not data:
                    break
                output += data
                
                # SSH password prompt
                if b"password:" in data.lower() and not ssh_pass_sent:
                    os.write(fd, (password + "\n").encode())
                    ssh_pass_sent = True
                    continue
                
                # Sudo password prompt (standard sudo -S prompt is often '[sudo] password for...')
                if b"password" in data.lower() and ssh_pass_sent and not sudo_pass_sent:
                    os.write(fd, (password + "\n").encode())
                    sudo_pass_sent = True
            except OSError:
                break
        
        os.close(fd)
        os.waitpid(pid, 0)
        return output.decode(errors='ignore')

user = "gemini"
host = "192.168.234.108"
password = "tuff"

print("--- [1] UEFI Boot Manager Detailed List ---")
print(run_ssh_sudo_command(user, host, password, "efibootmgr -v"))

print("\n--- [2] Inspecting EFI System Partition Content ---")
cmds = [
    "mkdir -p /tmp/esp",
    "mount /dev/nvme0n1p3 /tmp/esp",
    "find /tmp/esp -maxdepth 3",
    "umount /tmp/esp"
]
for c in cmds:
    print(f"Executing: {c}")
    print(run_ssh_sudo_command(user, host, password, c))

print("\n--- [3] Checking for MX Linux Root Filesystem ---")
print(run_ssh_sudo_command(user, host, password, "ls /dev/nvme0n1p4"))
