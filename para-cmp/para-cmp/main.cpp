#include <future>
#include <iostream>
#include <thread>
#include <optional>
#include <vector>
#include <fstream>
#include <memory>
#include <string>
#include <cstring>
#include <mutex>
#include <filesystem>
#include <chrono>
#include <atomic>
#include <filesystem>

//#define MEASUREMENTS
//#define DEBUG


#ifdef _WIN32
#ifdef MEASUREMENTS
#include <Windows.h>

void disable_file_caching(const char* fn) {
	HANDLE hd = CreateFileA(fn, GENERIC_READ, 0, NULL, OPEN_EXISTING, FILE_FLAG_NO_BUFFERING, NULL);
	CloseHandle(hd);
}

// god, this is annoying
#undef min
#endif
#endif

using ComparisonResult = std::optional<std::size_t>;

template <typename T>
void log(const T& var) {
	static std::mutex logMutex;
	std::lock_guard lock(logMutex);
	std::cerr << std::this_thread::get_id() << ' ' << var << '\n';
}

struct ComparisonParams {
	std::size_t wholeChunkSize;
	std::size_t bufferSize;
	std::size_t startOffset;
};

const std::atomic<std::size_t> totalRead;

// initialize -> (read -> compare) loop
class FileChunk {
	std::vector<char> buffer1;
	std::vector<char> buffer2;

	std::ifstream fileStream1;
	std::ifstream fileStream2;

	std::size_t currentReadOffset{ 0 };
	std::size_t lastReadOffset{ 0 };
	const ComparisonParams params;
	bool ok{ true };

public:
	FileChunk(const std::filesystem::path& file1, const std::filesystem::path& file2, const ComparisonParams& params)
		: fileStream1(file1, std::ios::binary), fileStream2(file2, std::ios::binary),
		params(params) {
		if (!fileStream1.is_open() || !fileStream2.is_open()) {
			ok = false;
		}

		fileStream1.seekg(params.startOffset);
		fileStream2.seekg(params.startOffset);

		buffer1.resize(params.bufferSize);
		buffer2.resize(params.bufferSize);

		if (!fileStream1 || !fileStream2) {
			ok = false;
		}
#ifdef DEBUG
		log("start at " + std::to_string(params.startOffset));
		log("chunk size " + std::to_string(params.wholeChunkSize));
#endif
	}

	bool isOk() const {
		return ok;
	}

	std::size_t getCurrentReadOffset() const {
		return currentReadOffset;
	}

	std::size_t getRemainingBytes() const {
		return params.wholeChunkSize - currentReadOffset;
	}

	bool isEnd() const {
		return getCurrentReadOffset() >= params.wholeChunkSize;
	}

	// reads up to configured buffer size bytes (returns actual read size or nullopt on error)
	std::optional<std::streamsize> read() {
		std::size_t size = std::min(params.bufferSize, getRemainingBytes());

		if (size == 0) {
			log("WARN: read size 0");
		}

		fileStream1.read(buffer1.data(), size);
		fileStream2.read(buffer2.data(), size);

		if (fileStream1.gcount() != fileStream2.gcount()) {
			log("ERROR: Files are of different length");
			return std::nullopt;
		}
		else if (fileStream1.gcount() != size) {
			log("WARN: File read read different size than requested");
		}
		lastReadOffset = currentReadOffset;
		currentReadOffset += fileStream1.gcount();
#ifdef MEASUREMENTS
		totalRead += fileStream1.gcount();
#endif
		return fileStream1.gcount();
	}

	// compares internal buffers, returns offset of first difference or nullopt if no difference found
	std::optional<std::streamsize> compare(std::size_t size) {
		if (memcmp(buffer1.data(), buffer2.data(), size) != 0) {
			for (size_t i = 0; i < fileStream1.gcount(); ++i) {
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
ComparisonResult compareFiles(const std::filesystem::path& file1, const std::filesystem::path& file2, const ComparisonParams& params) {
	using namespace std;

	FileChunk fileChunk(file1, file2, params);
	if (!fileChunk.isOk()) {
		return nullopt;
	}

	while (!fileChunk.isEnd()) {
		auto readResult = fileChunk.read();

		if (!readResult) {
			log("WARN: Files are of different length");
			return params.startOffset;
		}

		if (*readResult == 0) {
			break;
		}

		auto compareResult = fileChunk.compare(*readResult);
		if (compareResult.has_value()) {
			return compareResult;
		}
	}

	return nullopt;
}

std::optional<std::size_t> parseNumArg(const char* str) {
	if (std::strlen(str) == 0 || std::strlen(str) > 15) {
		log("Invalid number");
		return std::nullopt;
	}
	for (auto&& c : std::string_view(str))
	{
		if (!std::isdigit(c)) {
			log("Invalid number");
			return std::nullopt;
		}
	}
	return std::stoul(str);
}

void printTimeStats(std::chrono::high_resolution_clock::time_point start) {

#ifdef MEASUREMENTS
	using namespace std;
	auto end = chrono::high_resolution_clock::now();
	auto duration = chrono::duration_cast<chrono::milliseconds>(end - start).count();
	log("Time: " + to_string(duration) + "ms");
	log("Total read: " + to_string(totalRead) + " bytes, " + to_string(totalRead / 1024 / 1024) + " MB");
#endif
}

int main(int argc, char** argv) {
	std::filesystem::path file1;
	std::filesystem::path file2;

	if (argc < 3) {
		std::cerr << "Usage: " << argv[0] << " <file1> <file2> [taskCount] [bytesAvailable]\n";
		return 1;
	}

	file1 = argv[1];
	file2 = argv[2];

	if (file1 == file2) {
		log("ERROR: Files are the same");
		return 0;
	}
	if (!std::filesystem::exists(file1)) {
		log("ERROR: File 1 does not exist");
		return 1;
	}
	else if (!std::filesystem::exists(file2)) {
		log("ERROR: File 2 does not exist");
		return 1;
	}

	std::size_t taskCount = std::thread::hardware_concurrency() * 4ULL;
	if (argc > 3) {
		auto res = parseNumArg(argv[3]).value_or(taskCount);
		if (!res) {
			log("ERROR: Invalid task count");
			return 1;
		}
		taskCount = res;
	}

	std::size_t availableBytes = 1ULL * 1024 * 1024;
	if (argc > 4) {
		auto res = parseNumArg(argv[4]).value_or(availableBytes);
		if (!res) {
			log("ERROR: Invalid available bytes");
			return 1;
		}
		availableBytes = res;
	}

	std::vector<std::future<ComparisonResult>> tasks;

	std::size_t fileSize1 = std::filesystem::file_size(file1);

	if (std::size_t fileSize2 = std::filesystem::file_size(file2);  fileSize1 != fileSize2) {
		log("ERROR: Files are of different length");
		return 1;
	}

	std::size_t memoryPerTask = availableBytes / taskCount;

	if (availableBytes % taskCount != 0 || memoryPerTask == 0 || memoryPerTask % 2 != 0) {
		log("ERROR: Available bytes is not divisible by (task count * 2)");
		return 1;
	}

#ifdef MEASUREMENTS
	disable_file_caching(file1.c_str());
	disable_file_caching(file2.c_str());

	log("Task count: " + std::to_string(taskCount));
	log("Memory per task: " + std::to_string(memoryPerTask));
#endif // MEASUREMENTS

	std::size_t chunkSize = (fileSize1 + taskCount - 1) / taskCount;

	std::chrono::high_resolution_clock::time_point start = std::chrono::steady_clock::now();

	for (std::size_t i = 0; i < taskCount; ++i) {
		std::size_t startOffset = i * chunkSize;
		std::size_t realChunkSize = std::min(chunkSize, fileSize1 - startOffset);

		tasks.push_back(
			std::async(std::launch::async,
				compareFiles, file1, file2, ComparisonParams
				{
					realChunkSize,
					memoryPerTask / 2,
					startOffset
				})
		);
	}

	for (const auto& task : tasks) {
		task.wait();
	}

	int ret = 0;
	for (auto& task : tasks) {
		auto result = task.get();
		if (result) {
			log(std::format("Files differ at offset {}", std::to_string(result.value())));
			ret = 1;
			break;
		}
	}

	printTimeStats(start);
	return ret;
}