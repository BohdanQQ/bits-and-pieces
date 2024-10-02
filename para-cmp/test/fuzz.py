import os
import random
import sys

FILENAME_1 = "fuzz_file_1"
FILENAME_2 = "fuzz_file_2"
OUT_FILE_NAME = "fuzz_output"
MAX_DIFF_SIZE = 512
EMPTY_RANGE = (-1, -1)

def check_file_exists(file_path):
  if not os.path.exists(file_path):
    print(f"Error: File '{file_path}' does not exist.")
    sys.exit(1)

def get_other_file(data):
    if random.randint(0, 1):
      return ( bytes([b for b in data]), (EMPTY_RANGE))
    
    # Insert random discrepancy inside the file, maintaining its size
    start_idx = random.randint(1, len(data)) - 1
    end_idx = random.randint(start_idx + 1, min(len(data), start_idx + MAX_DIFF_SIZE))
    data = data[:start_idx] + os.urandom(end_idx - start_idx) + data[end_idx:]
    return (data, (start_idx, end_idx))

def generate_random_data(file_size):
  
  data = os.urandom(file_size)
  (other_data, diff_range) = get_other_file(data)
  
  with open(FILENAME_1, "wb") as f:
    f.write(data)
  
  with open(FILENAME_2, "wb") as f:
    f.write(other_data)
  
  return diff_range

def check_output(expected_range):
  if expected_range == EMPTY_RANGE:
    return True
  
  (start_idx, end_idx) = expected_range
  with open(OUT_FILE_NAME, "r") as f:
    for line in f:
      if "offset" in line:
        try:
          offset = int(line.split()[-1])
          if offset >= end_idx or offset < start_idx:
            print(f"Error: Offset mismatch. Expected number in {expected_range}, got {offset}")
            return False
        except:
          print("Warning: invalid format of the line:")
          print(line)
    
    return True

def clear_files():
  for filename in [FILENAME_1, FILENAME_2, OUT_FILE_NAME]:
    try:
      os.remove(filename)
    except:
      pass

def fuzz(num_passes, file_path, mem, thread_count):
  for i in range(num_passes):
    clear_files()
    diff_range = generate_random_data(file_size)
    
    code = os.system(f"{file_path} {FILENAME_1} {FILENAME_2} {thread_count} {mem} 2> {OUT_FILE_NAME}")
    
    if code != 0 and diff_range == EMPTY_RANGE:
      print(f"Error: Test failed on pass {i} (expected same files)")
      return False
    elif code == 0 and diff_range != EMPTY_RANGE:
      print(f"Error: Test failed on pass {i} (expected different files)")
      print(f"  index of the injected discrepancy: {diff_range}")
      return False

    if diff_range != EMPTY_RANGE and not check_output(diff_range):
      return False

  return True
      

if __name__ == "__main__":
  if len(sys.argv) < 4:
    print("Usage: python fuzz.py <file_path> <seed> <file_size> <num_passes> <thread_count> <memory per thread> ")
    sys.exit(1)

  file_path = sys.argv[1]
  seed = int(sys.argv[2])
  file_size = int(sys.argv[3])
  num_passes = int(sys.argv[4])
  thread_count = int(sys.argv[5])
  memory = 4096 * 4096
  try:
    memory = int(sys.argv[6]) * thread_count
    if memory % (thread_count * 2) != 0:
      print("total memory must be divisible by threadcoutn * 2")
      exit(1)
  except:
    pass

  random.seed(seed)

  check_file_exists(file_path)
  if fuzz(num_passes, file_path, memory, thread_count):
    clear_files()
    exit(0)

  print("Error: Test failed.")
  print("Arguments:")
  print(f"  file_path: {file_path}")
  print(f"  seed: {seed}")
  print(f"  file_size: {file_size}")
  print(f"  num_passes: {num_passes}")
  print(f"  memory: {memory}")
  print(f"  thread_count: {thread_count}")

  print("Check the files:")
  print(f"{FILENAME_1}")
  print(f"{FILENAME_2}")
  print(f"{OUT_FILE_NAME}")

  sys.exit(1)
  