import errno
import fcntl
import sys


with open(sys.argv[1], "a") as lock:
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError as error:
        if error.errno not in (errno.EACCES, errno.EAGAIN):
            raise
        print(f"Coverage build directory is already in use: {sys.argv[1]}", file=sys.stderr)
        sys.exit(73)
    print("locked", flush=True)
    sys.stdin.buffer.read()
