#ifndef BQQ_DUMP_HPP
#define BQQ_DUMP_HPP

#include <fstream>
#include <string>
#include <span>
#include <concepts>

namespace bqq_fileutils
{
  template <typename T, typename S>
  concept into_span_of = requires(T a) {
    std::span<S>(a);
  };

  template <typename T>
  concept has_value_type = requires(T) {
    std::declval<typename T::value_type>();
  };

  template <typename T, std::size_t SIZE>
  concept iterable_of_item_size =
      has_value_type<T> &&
      std::copyable<typename T::value_type> &&
      std::is_trivially_copyable_v<typename T::value_type> &&
      sizeof(typename T::value_type) == SIZE &&
      (into_span_of<T, const std::byte> || into_span_of<T, const std::uint8_t> ||
       into_span_of<T, const std::int8_t> || into_span_of<T, const char>);

  template <iterable_of_item_size<1> T>
  void dump(const T &data, const std::string &filename) {
    std::ofstream file(filename, std::ios::binary);
    file.write(reinterpret_cast<const char *>(data.data()), data.size());
  }
}

#endif // BQQ_DUMP_HPP