# cpp-toolbox

Various C++ utilities.

## `defes.hpp`

Defines abbreviated versions of some standard types.

Examples:

`std::uint8_t` -> `u8`
`std::uint16_t` -> `u16`
`std::int64_t` -> `i64`

## `dump.hpp`

Basic file dump utility for span-able types.

Example usage:

```c++
#include "include/dump.hpp"
#include <vector>

int main() {
  std::string filename = "test.bin";
  
  std::vector<std::byte> data = {std::byte{0x00}, std::byte{0x01}, std::byte{0x02}, std::byte{0x03}};
  
  bqq_fileutils::dump(std::span(data), filename);
  
  return 0;
}
```

## `move.hpp`

Defines a special, kinda-safer version of `std::move`.

Adopted from [Jonathan Müller's talk on CPPCon 2024](https://www.youtube.com/watch?v=hvnl6T2MnUk)

I think of this mostly as a sanity-checking tool - i.e. before commiting your changes,
replace std::move with MOVE and see if it still compiles. If not, try to address the errors (can be spurious).
