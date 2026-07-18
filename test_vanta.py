import pty
import os
import time

pid, fd = pty.fork()
if pid == 0:
    os.execvp("cargo", ["cargo", "run"])
else:
    output = b""
    try:
        # Wait a bit for it to build and start
        time.sleep(5)
        os.write(fd, b'q')
        time.sleep(1)
        os.write(fd, b'q')
        while True:
            import select
            r, _, _ = select.select([fd], [], [], 1.0)
            if not r: break
            data = os.read(fd, 4096)
            if not data: break
            output += data
    except Exception as e:
        print(e)
    os.waitpid(pid, 0)
    
    # parse the output for panics
    if b"panicked" in output:
        print("Panic found:")
        idx = output.find(b"panicked")
        print(output[max(0, idx-50):idx+200].decode('utf-8', 'replace'))
    else:
        print("No panics detected. Output size:", len(output))

