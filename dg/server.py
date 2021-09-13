import multiprocessing
import sys
import time
from concurrent.futures import ProcessPoolExecutor

from worker import Worker

if __name__ == "__main__":
    num_workers = None
    try:
        num_workers = int(sys.argv[1])
    except:
        num_workers = multiprocessing.cpu_count()

    start_port = 6000
    print("num_workers=", num_workers)
    pool = ProcessPoolExecutor(max_workers=num_workers)
    workers = [Worker(port) for port in range(start_port, start_port + num_workers)]
    for w in workers:
        pool.submit(w.start)

    try:
        while True:
            time.sleep(1)
    except:
        print("interrupted")
        for w in workers:
            w.close()
        if pool:
            print("killing pool")
            pool.shutdown(False)


    print("terminated -> to return to console: ctrl + break")
