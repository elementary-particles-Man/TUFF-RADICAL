import pty
import os
import sys
import subprocess
import time

def run_ssh_command(user, host, password, command):
    # ssh command with password prompt handling
    cmd = ['ssh', '-o', 'StrictHostKeyChecking=no', f'{user}@{host}', command]
    
    pid, fd = pty.fork()
    
    if pid == 0:
        # Child process
        os.execvp('ssh', cmd)
    else:
        # Parent process
        output = b""
        password_sent = False
        
        while True:
            try:
                data = os.read(fd, 1024)
                if not data:
                    break
                output += data
                
                if b"password:" in data and not password_sent:
                    os.write(fd, (password + "\n").encode())
                    password_sent = True
            except OSError:
                break
        
        os.close(fd)
        _, exit_status = os.waitpid(pid, 0)
        return output.decode(errors='ignore')

user = "gemini"
host = "192.168.234.108"
password = "tuff"

print("--- System Inventory (lsblk) ---")
print(run_ssh_command(user, host, password, "lsblk -f"))

print("\n--- UEFI Boot Manager (efibootmgr) ---")
print(run_ssh_command(user, host, password, "sudo efibootmgr -v"))

print("\n--- NVMe Smart Status ---")
print(run_ssh_command(user, host, password, "sudo smartctl -a /dev/nvme0n1 | grep -E 'Critical Warning|Integrity Errors'"))
