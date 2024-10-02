#include <atomic>
#include <chrono>
#include <cstring>
#include <filesystem>
#include <format>
#include <fstream>
#include <future>
#include <iostream>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <thread>
#include <vector>

// #define MEASUREMENTS
// #define DEBUG

#ifdef _WIN32
#ifdef MEASUREMENTS
#include <Windows.h>
#include <comdef.h>

// this sometimes does not work...???
static void disable_file_caching(const wchar_t* fn) {
    // Constructs a _bstr_t object by calling SysAllocString to create a new
    // BSTR object and then encapsulates it.
    _bstr_t b(fn);
    // Convert to char*
    const char* c = b;

    HANDLE hd = CreateFileA(c, GENERIC_READ, 0, NULL, OPEN_EXISTING,
                            FILE_FLAG_NO_BUFFERING, NULL);
    CloseHandle(hd);
}

// god, this is annoying
#undef min
#endif
#endif

#ifdef MEASUREMENTS
struct PaddedCount {
    __declspec(align(64)) std::streamsize value;
};

std::vector<PaddedCount> readCounts;
#endif

static inline void registerRead([[maybe_unused]] std::size_t threadIdx,
                                [[maybe_unused]] std::streamsize size) {
#ifdef MEASUREMENTS
    // readCounts could be made function-local-static but I cannot be bothered
    // to look up the thread safety of such initialization
    readCounts[threadIdx].value += size;
#endif
}

static inline std::streamsize getTotalRead() {
    std::streamsize totalRead = 0;
#ifdef MEASUREMENTS
    for (auto const& threadReadCount : readCounts) {
        totalRead += threadReadCount.value *
                     2;   // reading from both files on each thread
    }
#endif
    return totalRead;
}

enum class Result { OkDiff, OkSame, Error };

using ComparisonResult = std::tuple<Result, std::size_t>;

constexpr int E_OK        = 0;
constexpr int E_DIFFERENT = 1;
constexpr int E_USAGE     = 2;
constexpr int E_OTHER     = 3;
// no idea why but for larger task count, file opening fails...
// realistically the number will remain small for the forseeable future
constexpr int MAX_TASK = 32;
constexpr int MIN_TASK = 2;

template <typename T>
void log(const T& var) {
    static std::mutex logMutex;
    std::lock_guard lock(logMutex);
    std::cerr << std::this_thread::get_id() << ' ' << var << '\n';
}

template <typename T>
void printTimeStats([[maybe_unused]] const T& start) {
#ifdef MEASUREMENTS
    using namespace std;
    auto end = chrono::high_resolution_clock::now();
    auto duration =
        chrono::duration_cast<chrono::milliseconds>(end - start).count();
    log(std::format("Time: {} ms", duration));

    auto totalRead = getTotalRead();
    log(std::format("Total read: {} bytes, {} MB", totalRead,
                    totalRead / 1024 / 1024));
#endif
}

struct ComparisonParams {
    std::size_t wholeChunkSize;
    std::size_t bufferSize;
    std::size_t startOffset;
    std::size_t threadId;
};

// initialize -> (read -> compare) loop
class FileChunk {
    std::vector<char> buffer1;
    std::vector<char> buffer2;

    std::ifstream fileStream1;
    std::ifstream fileStream2;

    std::size_t currentReadOffset {0};
    std::size_t lastReadOffset {0};
    const ComparisonParams params;
    bool ok {true};

   public:
    FileChunk(const std::filesystem::path& file1,
              const std::filesystem::path& file2,
              const ComparisonParams& params)
        : fileStream1(file1, std::ios::binary),
          fileStream2(file2, std::ios::binary),
          params(params) {
        ok &= fileStream1.is_open() && fileStream2.is_open();
        if (! ok) {
#ifdef DEBUG
            log("chunk error: failed to open");
#endif
            return;
        }

        fileStream1.seekg(params.startOffset);
        fileStream2.seekg(params.startOffset);

        buffer1.resize(params.bufferSize);
        buffer2.resize(params.bufferSize);

        ok &= fileStream1 && fileStream2;
#ifdef DEBUG
        if (! ok) {
            log(
                std::format("chunk error: ctor cannot seekg on stream/other "
                            "stream error {}",
                            (fileStream1 ? 1 : 2)));
        }
        log(std::format("start at {}", params.startOffset));
        log(std::format("chunk size {}", params.wholeChunkSize));
        log(std::format("buff size {}", params.bufferSize));
#endif
    }

    bool isOk() const { return ok; }

    std::size_t getCurrentReadOffset() const { return currentReadOffset; }

    std::size_t getRemainingBytes() const {
        return params.wholeChunkSize - currentReadOffset;
    }

    bool isEnd() const {
        return getCurrentReadOffset() >= params.wholeChunkSize;
    }

    // reads up to configured buffer size bytes (returns actual read size or
    // nullopt on error)
    std::optional<std::streamsize> read() {
        std::size_t size = std::min(params.bufferSize, getRemainingBytes());

        if (size == 0) {
            log("WARN: read size 0");
        }

        fileStream1.read(buffer1.data(), size);
        fileStream2.read(buffer2.data(), size);
        auto fs1Count = fileStream1.gcount();
        if (auto fs2Count = fileStream2.gcount(); fs1Count != fs2Count) {
            log(std::format(
                "ERROR: Files are of different length (1: {} vs 2: {})",
                fs1Count, fs2Count));
            return std::nullopt;
        } else if (fs1Count != size) {
            log(std::format(
                "WARN: File read count ({}) different size than requested ({})",
                fs1Count, size));
        }
        lastReadOffset = currentReadOffset;
        currentReadOffset += fs1Count;
        registerRead(params.threadId, fs1Count);
        return fs1Count;
    }

    // compares internal buffers, returns offset of first difference or nullopt
    // if no difference found
    std::optional<std::streamsize> compare(std::size_t size) {
        if (memcmp(buffer1.data(), buffer2.data(), size) != 0) {
            // watch out, the gcount call here implies no other reads on the filestream1 can take place
            // before calling this funciton
            for (std::streamsize i = 0; i < fileStream1.gcount(); ++i) {
                if (buffer1[i] != buffer2[i]) {
                    log("FOUND!");
                    return params.startOffset + lastReadOffset + i;
                }
            }
            log("BUG! Comparison failed");
            return params.startOffset;
        }
        return std::nullopt;
    }
};

// nullopt if no difference
static ComparisonResult compareFiles(const std::filesystem::path& file1,
                                     const std::filesystem::path& file2,
                                     const ComparisonParams& params) {
    using namespace std;

    FileChunk fileChunk(file1, file2, params);
    if (! fileChunk.isOk()) {
        log("ERROR: Not \"OK\" chunk... cannot compare");
        return {Result::Error, 0};
    }

    while (! fileChunk.isEnd()) {
        auto readResult = fileChunk.read();

        if (! readResult) {
            log("WARN: Files are of different length");
            return {Result::OkDiff, params.startOffset};
        }

        if (*readResult == 0) {
            log("WARN: Skip");
            break;
        }

        auto compareResult = fileChunk.compare(*readResult);
        if (compareResult.has_value()) {
            return {Result::OkDiff, *compareResult};
        }
    }

    return {Result::OkSame, params.startOffset};
}

static std::optional<std::size_t> parseNumArg(const char* str) {
    if (std::strlen(str) == 0 || std::strlen(str) > 15) {
        log("Invalid number");
        return std::nullopt;
    }
    for (auto&& c : std::string_view(str)) {
        if (! std::isdigit(c)) {
            log("Invalid number");
            return std::nullopt;
        }
    }
    return std::stoul(str);
}

struct Args {
    std::filesystem::path file1;
    std::filesystem::path file2;
    std::size_t taskCount      = 2ULL;
    std::size_t availableBytes = taskCount * 4096;
};

static std::optional<Args> parseArgs(int argc, char** argv) {
    Args res;
    std::optional<Args> err = std::nullopt;

    if (argc < 3) {
        log(std::format(
            "Usage: {} <file1> <file2> [taskCount {}:{}] [bytesAvailable]",
            argv[0], MIN_TASK, MAX_TASK));
        log("Status code 0 - files are the same\n1 - files differ\n2 - usage "
            "error\n3 - other error (usually file errors)");
        return err;
    }

    res.file1 = argv[1];
    res.file2 = argv[2];

    if (! std::filesystem::exists(res.file1)) {
        log("ERROR: File 1 does not exist");
        return err;
    } else if (! std::filesystem::exists(res.file2)) {
        log("ERROR: File 2 does not exist");
        return err;
    }

    if (argc > 3) {
        auto num = parseNumArg(argv[3]).value_or(res.taskCount);
        if (! num) {
            log("ERROR: Invalid task count");
            return err;
        }

        if (num < MIN_TASK || num > MAX_TASK) {
            log(std::format("ERROR: task count allowed only from [{};{}]",
                            MIN_TASK, MAX_TASK));
            return err;
        }
        res.taskCount = num;
    }

    if (argc > 4) {
        auto num = parseNumArg(argv[4]).value_or(res.availableBytes);
        if (! num) {
            log("ERROR: Invalid available bytes");
            return err;
        }
        res.availableBytes = num;
    }

    return res;
}

int main(int argc, char** argv) {
    auto args = parseArgs(argc, argv);
    if (! args) {
        return E_USAGE;
    }

    auto&& [file1, file2, taskCount, availableBytes] = *args;

    if (file1 == file2) {
        return E_OK;
    }

    std::vector<std::future<ComparisonResult>> tasks;

    std::size_t fileSize1 = std::filesystem::file_size(file1);

    if (std::size_t fileSize2 = std::filesystem::file_size(file2);
        fileSize1 != fileSize2) {
        log("ERROR: Files are of different length");
        return E_USAGE;
    }

    std::size_t memoryPerTask = availableBytes / taskCount;

    if (availableBytes % taskCount != 0 || memoryPerTask == 0 ||
        memoryPerTask % 2 != 0) {
        log("ERROR: Available bytes is not divisible by (task count * 2)");
        return E_USAGE;
    }

#ifdef MEASUREMENTS
    disable_file_caching(file1.c_str());
    disable_file_caching(file2.c_str());

    log(std::format("Task count: {}", taskCount));
    log(std::format("Memory per task: {}", memoryPerTask));
    readCounts.resize(taskCount);
#endif   // MEASUREMENTS

    std::size_t chunkSize = (fileSize1 + taskCount - 1) / taskCount;

    auto start = std::chrono::steady_clock::now();

    for (std::size_t i = 0; i < taskCount; ++i) {
        std::size_t startOffset = i * chunkSize;
        if (startOffset >= fileSize1) {
            break;
        }

        std::size_t realChunkSize =
            std::min(chunkSize, fileSize1 - startOffset);

        tasks.push_back(
            std::async(std::launch::async, compareFiles, file1, file2,
                       ComparisonParams {realChunkSize, memoryPerTask / 2,
                                         startOffset, i}));
    }

    for (const auto& task : tasks) {
        task.wait();
    }

    int ret = E_OK;
    for (auto& task : tasks) {
        using enum Result;
        auto&& [status, offset] = task.get();
        if (status == OkDiff) {
            log(std::format("Files differ at offset {}", offset));
            ret = E_DIFFERENT;
        } else if (status == Error) {
            ret = E_OTHER;
        }

        if (status != OkSame) {
            break;
        }
    }

    printTimeStats(start);
    return ret;
}