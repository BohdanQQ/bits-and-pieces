
#ifndef BQQ_MOVE_HPP
#define BQQ_MOVE_HPP

/*
 * A replacement for std::move that checks for various pitfalls
 *
 * Recommended usage: replace std::move with MOVE
 *
 * Adopted from https://www.youtube.com/watch?v=hvnl6T2MnUk
 * (Jonathan Müller's talk in CPPCon 2024)
 */

template <typename DeclTypeT, bool IsIdExpression, typename T>
constexpr std::remove_reference_t<T>&& move_impl(T&& obj) noexcept {
    static_assert(! std::is_const_v<std::remove_reference_t<T>>,
                  "Cannot move const object");
    static_assert(! std::is_lvalue_reference_v<DeclTypeT>,
                  "Cannot move lvalue, consider copying");
    static_assert(
        IsIdExpression,
        "Don't write MOVE(obj.member), write MOVE(obj).member instead");

    return static_cast<std::remove_reference_t<T>&&>(obj);
}

// std::isalpha and std::isdigit are not constexpr in C++20
#define move_help_isdigit(c) ((c) >= '0' && (c) <= '9')
#define move_help_isalpha(c) \
    ((c) >= 'a' && (c) <= 'z' || (c) >= 'A' && (c) <= 'Z')

constexpr bool is_id_expression_impl(const char* const expr) noexcept {
    for (auto str = expr; *str; ++str) {
        if (! move_help_isalpha(*str) && ! move_help_isdigit(*str) &&
            *str != '_' && *str != ':') {
            return false;
        }
    }
    return true;
}

#define is_id_expression(...) is_id_expression_impl(#__VA_ARGS__)
#define MOVE(...) \
    move_impl<decltype(__VA_ARGS__), is_id_expression(__VA_ARGS__)>(__VA_ARGS__)

#endif   // BQQ_MOVE_HPP