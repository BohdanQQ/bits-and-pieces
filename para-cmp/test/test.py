import os
import random
import sys

TEST_SEED = 12345678

def call_on_files(binary_path, file1, file2, thread_count, memory):
      OUT_FILE_NAME = "test-result-out"
      code = os.system(f"{binary_path} {file1} {file2} {thread_count} {memory} 2> {OUT_FILE_NAME}")
      return (code, OUT_FILE_NAME)

def create_random_data(size):
  return random.randbytes(size)

def write_data(data, file):
   with open(file, "wb") as f:
      f.write(data)

FILE1 = "test-file1"
FILE2 = "test-file2"

def test_file(size, binary_path, thread_count, memory):

  d1 = create_random_data(size)
  write_data(d1, FILE1)
  write_data(d1, FILE2)

  (code, out_file_name) = call_on_files(binary_path, FILE1, FILE2, thread_count, memory)


  if code == 0:
     return True
  

  result = open(out_file_name).readlines()

  print("Test SAME failed, unexpected output:")
  for l in result:
     print(l)
  
  return False   

def test_same(binary_path, thread_count, memory):
   return test_file(4096, binary_path, thread_count, memory)


def test_same_small(binary_path, thread_count, memory):
  SMALL_SIZE_MIN = 1
  SMALL_SIZE_MAX = 128

  for i in range(SMALL_SIZE_MIN, SMALL_SIZE_MAX):
     if not test_file(i, binary_path, thread_count, memory):
        print("Test failed on file size " + i)
        return False

  return True

def eval_different(expected, output_lines):
  detected = False
  for line in output_lines:  
    if "offset" in line:
      try:
        offset = int(line.split()[-1])
        if offset != expected:
          print(f"Error: Offset mismatch. Expected offset: {expected}, offset gotten: {offset}")
          return False
        else:
           detected = True
      except:
        print("Warning: invalid format of the line:")
        print(line)
    elif "ERROR" in line:
      print("Warning: invalid format of the line:")
      print(line)
  return detected


def index_test_one_byte(size, start, end, binary_path, thread_count, memory):
  d1 = create_random_data(size)
  write_data(d1, FILE1)
  
  for i in range(start, end):
    d2 = bytes([b for b in d1])
    d2 = d2[:i] + (bytes('\x01', 'ascii') if d2[i] == bytes('\x00', 'ascii')[0] else bytes('\x00', 'ascii')) + d2[i+1:]
    write_data(d2, FILE2)

    (code, output_file) = call_on_files(binary_path, FILE1, FILE2, thread_count, memory)
    if code == 0:
      print("Unexpected: binary reports files are same")
      print("size " + str(size) + " start " + str(start) + " end " + str(end))
      print(i)
      return False

    if not eval_different(i, open(output_file, "r").readlines()):
      return False

  return True
     
def test_detect_start(binary_path, thread_count, memory):
   return index_test_one_byte(1024, 0, 128, binary_path, thread_count, memory)

def test_detect_end(binary_path, thread_count, memory):
   return index_test_one_byte(1024, 1024-128, 1024, binary_path, thread_count, memory)

def test_detect_mid(binary_path, thread_count, memory):
   return index_test_one_byte(1024, 512-64, 512+64, binary_path, thread_count, memory)


if __name__ == "__main__":
  random.seed(TEST_SEED)
  binary_path = sys.argv[1]
  thread_count = int(sys.argv[2])
  memory = thread_count * 2 * 1024

  for t in [
      test_same,
      test_same_small,
      test_detect_start,
      test_detect_end,
      test_detect_mid
    ]:
     if not t(binary_path, thread_count, memory):
        exit(1)
  
  exit(0)
