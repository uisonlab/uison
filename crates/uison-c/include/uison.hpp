/*
 * uison.hpp — C++ on top of uison.h: RAII, lambdas, data <-> C++ types, a screen builder.
 * Header-only. C++17 is the base; C++20 adds aggregate reflection (structs <-> data).
 *
 *   #include <uison.hpp>
 *   int main() {
 *       uison::Ui ui(R"json({ data: { n: 0 }, root: { Button: { text: "+1 ({@n})", on: { click: "inc" } } } })json");
 *       ui.on("inc", [](uison::Cx& cx) { cx.data().set("n", cx.data().get<int>("n") + 1); });
 *       return ui.run("Counter");
 *   }
 *
 * JSON in code — as a tagged raw string: R"json( … )json" (JSON often contains `)"`).
 */
#ifndef UISON_HPP
#define UISON_HPP

#include <array>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <functional>
#include <initializer_list>
#include <map>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <tuple>
#include <type_traits>
#include <utility>
#include <vector>

#include "uison.h"

#if defined(_MSVC_LANG) && _MSVC_LANG > __cplusplus
#define UISON_CPP _MSVC_LANG
#else
#define UISON_CPP __cplusplus
#endif
#if UISON_CPP >= 202002L
#define UISON_REFLECT 1
#else
#define UISON_REFLECT 0
#endif

namespace uison {

/// Error parsing a screen, components or JSON.
struct Error : std::runtime_error {
    using std::runtime_error::runtime_error;
};

class Data;
class Node;
struct obj;
struct arr;

/// Custom type conversion: `template <> struct uison::convert<My> { static void write(std::string& out, const My&);
/// static My read(const uison::Data&, const std::string& path); };`
template <class T, class = void>
struct convert;

namespace detail {

/// NUL-terminated path: `std::string_view` from C++ → `const char*` for C.
class Path {
    std::string s_;

public:
    Path(std::string_view p) : s_(p) {}
    const char* c() const { return s_.c_str(); }
};

inline std::string join(const std::string& path, std::string_view field) {
    std::string r = path;
    if (!r.empty()) r += '.';
    r += field;
    return r;
}

inline void write_string(std::string& out, std::string_view s) {
    out += '"';
    for (char ch : s) {
        auto c = static_cast<unsigned char>(ch);
        switch (c) {
            case '"': out += "\\\""; break;
            case '\\': out += "\\\\"; break;
            case '\n': out += "\\n"; break;
            case '\r': out += "\\r"; break;
            case '\t': out += "\\t"; break;
            default:
                if (c < 0x20) {
                    char buf[8];
                    std::snprintf(buf, sizeof buf, "\\u%04x", c);
                    out += buf;
                } else {
                    out += ch;
                }
        }
    }
    out += '"';
}

/// Number without locale: integers as integers, the rest in the shortest form of %.15g / %.17g.
inline void write_number(std::string& out, double v) {
    if (!std::isfinite(v)) {
        out += "null";
        return;
    }
    char buf[40];
    if (v == std::floor(v) && std::fabs(v) < 9007199254740992.0) {
        std::snprintf(buf, sizeof buf, "%lld", static_cast<long long>(v));
    } else {
        std::snprintf(buf, sizeof buf, "%.15g", v);
        if (std::strtod(buf, nullptr) != v) std::snprintf(buf, sizeof buf, "%.17g", v);
        for (char* p = buf; *p; p++)
            if (*p == ',') *p = '.'; // the locale's decimal comma
    }
    out += buf;
}

template <class T>
struct is_vector : std::false_type {};
template <class T, class A>
struct is_vector<std::vector<T, A>> : std::true_type {};
template <class T>
struct is_optional : std::false_type {};
template <class T>
struct is_optional<std::optional<T>> : std::true_type {};
template <class T>
struct is_map : std::false_type {};
template <class V, class C, class A>
struct is_map<std::map<std::string, V, C, A>> : std::true_type {};

template <class T, class = void>
struct has_convert : std::false_type {};
template <class T>
struct has_convert<T, std::void_t<decltype(convert<T>::write)>> : std::true_type {};

#if UISON_REFLECT
// ---- aggregate reflection (C++20): number of fields, pointers to fields, field names

// Only for checking "can the aggregate be initialized with N values" (in requires, never called) —
// hence no body and no constexpr (otherwise implicitly inline, and the compiler expects a definition).
struct any_field {
    template <class T>
    operator T&() const&& noexcept;
    template <class T>
    operator T&&() const&& noexcept;
};

template <class T, class... A>
constexpr std::size_t count_fields() {
    if constexpr (requires { T{A{}..., any_field{}}; })
        return count_fields<T, A..., any_field>();
    else
        return sizeof...(A);
}

template <class T>
constexpr bool reflectable = std::is_aggregate_v<T> && !std::is_array_v<T>;

/// Pointers to the object's fields (via structured bindings).
template <class T>
constexpr auto field_ptrs(const T& t) {
    constexpr std::size_t N = count_fields<T>();
    if constexpr (N == 1) {
        auto& [m0] = t;
        return std::tuple{&m0};
    } else if constexpr (N == 2) {
        auto& [m0, m1] = t;
        return std::tuple{&m0, &m1};
    } else if constexpr (N == 3) {
        auto& [m0, m1, m2] = t;
        return std::tuple{&m0, &m1, &m2};
    } else if constexpr (N == 4) {
        auto& [m0, m1, m2, m3] = t;
        return std::tuple{&m0, &m1, &m2, &m3};
    } else if constexpr (N == 5) {
        auto& [m0, m1, m2, m3, m4] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4};
    } else if constexpr (N == 6) {
        auto& [m0, m1, m2, m3, m4, m5] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5};
    } else if constexpr (N == 7) {
        auto& [m0, m1, m2, m3, m4, m5, m6] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6};
    } else if constexpr (N == 8) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7};
    } else if constexpr (N == 9) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8};
    } else if constexpr (N == 10) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9};
    } else if constexpr (N == 11) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10};
    } else if constexpr (N == 12) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11};
    } else if constexpr (N == 13) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12};
    } else if constexpr (N == 14) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13};
    } else if constexpr (N == 15) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14};
    } else if constexpr (N == 16) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15};
    } else if constexpr (N == 17) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16};
    } else if constexpr (N == 18) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17};
    } else if constexpr (N == 19) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18};
    } else if constexpr (N == 20) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19};
    } else if constexpr (N == 21) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19, &m20};
    } else if constexpr (N == 22) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20, m21] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19, &m20, &m21};
    } else if constexpr (N == 23) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20, m21, m22] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19, &m20, &m21, &m22};
    } else if constexpr (N == 24) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20, m21, m22, m23] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19, &m20, &m21, &m22, &m23};
    } else if constexpr (N == 25) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20, m21, m22, m23, m24] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19, &m20, &m21, &m22, &m23, &m24};
    } else if constexpr (N == 26) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20, m21, m22, m23, m24, m25] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19, &m20, &m21, &m22, &m23, &m24, &m25};
    } else if constexpr (N == 27) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20, m21, m22, m23, m24, m25, m26] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19, &m20, &m21, &m22, &m23, &m24, &m25, &m26};
    } else if constexpr (N == 28) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20, m21, m22, m23, m24, m25, m26, m27] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19, &m20, &m21, &m22, &m23, &m24, &m25, &m26, &m27};
    } else if constexpr (N == 29) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20, m21, m22, m23, m24, m25, m26, m27, m28] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19, &m20, &m21, &m22, &m23, &m24, &m25, &m26, &m27, &m28};
    } else if constexpr (N == 30) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20, m21, m22, m23, m24, m25, m26, m27, m28, m29] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19, &m20, &m21, &m22, &m23, &m24, &m25, &m26, &m27, &m28, &m29};
    } else if constexpr (N == 31) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20, m21, m22, m23, m24, m25, m26, m27, m28, m29, m30] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19, &m20, &m21, &m22, &m23, &m24, &m25, &m26, &m27, &m28, &m29, &m30};
    } else if constexpr (N == 32) {
        auto& [m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15, m16, m17, m18, m19, m20, m21, m22, m23, m24, m25, m26, m27, m28, m29, m30, m31] = t;
        return std::tuple{&m0, &m1, &m2, &m3, &m4, &m5, &m6, &m7, &m8, &m9, &m10, &m11, &m12, &m13, &m14, &m15, &m16, &m17, &m18, &m19, &m20, &m21, &m22, &m23, &m24, &m25, &m26, &m27, &m28, &m29, &m30, &m31};
    } else {
        static_assert(N <= 32, "uison.hpp: aggregates with more than 32 fields are not supported");
        return std::tuple<>{};
    }
}

// Only the address (the field name comes from the template signature) — a declaration without a definition.
template <class T>
extern const T ext;

// T as well, not only the address: MSVC before 19.5 (VS 2022) treats &ext<A>->x and &ext<B>->y at the same offset
// as one template argument, and B got the field names of A.
template <class T, auto P>
constexpr std::string_view signature() {
#if defined(_MSC_VER) && !defined(__clang__)
    return __FUNCSIG__;
#else
    return __PRETTY_FUNCTION__;
#endif
}

/// Template argument from the signature: clang "[T = T, P = &ext<T>.title]", GCC "[with T = T; auto P = (& ext<T>.T::title); …]",
/// MSVC «signature<struct T,&ext<struct T>->title>(void)» (the last identifier is the field anyway).
constexpr std::string_view template_arg(std::string_view s) {
    if (auto p = s.find("P = "); p != std::string_view::npos) {
        s = s.substr(p + 4);
        if (auto e = s.find(';'); e != std::string_view::npos) return s.substr(0, e);
        if (auto e = s.rfind(']'); e != std::string_view::npos) return s.substr(0, e);
        return s;
    }
    if (auto p = s.find("signature<"); p != std::string_view::npos) {
        s = s.substr(p + 10);
        if (auto e = s.rfind(">("); e != std::string_view::npos) return s.substr(0, e);
    }
    return s;
}

/// The field name is the last identifier in the template argument (a pointer to that field).
constexpr std::string_view trailing_identifier(std::string_view s) {
    s = template_arg(s);
    auto ident = [](char c) { return c == '_' || (c >= '0' && c <= '9') || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'); };
    std::size_t end = s.size();
    while (end > 0 && !ident(s[end - 1])) end--;
    std::size_t begin = end;
    while (begin > 0 && ident(s[begin - 1])) begin--;
    return s.substr(begin, end - begin);
}

#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wundefined-var-template" // ext<T> deliberately has no definition
#endif
template <class T, std::size_t I>
constexpr std::string_view field_name() {
    return trailing_identifier(signature<T, std::get<I>(field_ptrs(ext<T>))>());
}
#if defined(__clang__)
#pragma clang diagnostic pop
#endif
#endif

template <class T>
void write(std::string& out, const T& v);

template <class Tuple, std::size_t... I>
void write_fields(std::string& out, const Tuple& ptrs, std::index_sequence<I...>, const std::string_view* names) {
    out += '{';
    ((out += (I ? "," : ""), write_string(out, names[I]), out += ':', write(out, *std::get<I>(ptrs))), ...);
    out += '}';
}

} // namespace detail

/// A JSON value (already serialized): numbers, strings, bool, null, objects, arrays, structs.
class Json {
    std::string s_ = "null";

public:
    Json() = default;
    Json(std::nullptr_t) {}
    Json(bool v) : s_(v ? "true" : "false") {}
    Json(const char* v) { detail::write_string(s_ = {}, v ? v : ""); }
    Json(std::string_view v) { detail::write_string(s_ = {}, v); }
    Json(const std::string& v) { detail::write_string(s_ = {}, v); }
    template <class T, std::enable_if_t<std::is_arithmetic_v<T> && !std::is_same_v<T, bool>, int> = 0>
    Json(T v) {
        s_.clear();
        detail::write_number(s_, static_cast<double>(v));
    }
    /// Any supported type: vector, map, optional, an aggregate (C++20), a custom convert<T>.
    template <class T, std::enable_if_t<!std::is_arithmetic_v<T> && !std::is_convertible_v<const T&, std::string_view> &&
                                            !std::is_same_v<T, Json> && !std::is_same_v<T, obj> && !std::is_same_v<T, arr> &&
                                            !std::is_same_v<T, Node> && !std::is_same_v<T, std::nullptr_t>,
                                        int> = 0>
    Json(const T& v) {
        s_.clear();
        detail::write(s_, v);
    }
    /// A ready JSON string (not validated).
    static Json raw(std::string json) {
        Json j;
        j.s_ = std::move(json);
        return j;
    }
    const std::string& str() const { return s_; }
    const char* c_str() const { return s_.c_str(); }
};

/// A value in the screen language right in the code — a selector, a function, a list of options:
/// `.color = uison::json(R"({ switch: "@many", cases: { true: "$theme.danger" }, default: "$theme.muted" })")`.
/// Extended JSON (unquoted keys); validated when the screen is built (`uison::Ui`).
inline Json json(std::string_view src) { return Json::raw(std::string(src)); }

/// Object: `uison::obj{{"id", 1}, {"title", "bread"}, {"done", false}}`.
struct obj {
    std::vector<std::pair<std::string, Json>> fields;
    obj(std::initializer_list<std::pair<std::string_view, Json>> f) {
        for (auto& [k, v] : f) fields.emplace_back(std::string(k), v);
    }
    operator Json() const {
        std::string s = "{";
        for (std::size_t i = 0; i < fields.size(); i++) {
            if (i) s += ',';
            detail::write_string(s, fields[i].first);
            s += ':';
            s += fields[i].second.str();
        }
        return Json::raw(s + '}');
    }
};

/// Array: `uison::arr{1, "two", uison::obj{{"x", 3}}}`.
struct arr {
    std::vector<Json> items;
    arr(std::initializer_list<Json> i) : items(i) {}
    operator Json() const {
        std::string s = "[";
        for (std::size_t i = 0; i < items.size(); i++) {
            if (i) s += ',';
            s += items[i].str();
        }
        return Json::raw(s + ']');
    }
};

namespace detail {
template <class T>
void write(std::string& out, const T& v) {
    if constexpr (std::is_same_v<T, Json>) {
        out += v.str();
    } else if constexpr (std::is_same_v<T, obj> || std::is_same_v<T, arr>) {
        out += static_cast<Json>(v).str();
    } else if constexpr (std::is_same_v<T, Node>) {
        out += v.json().str();
    } else if constexpr (has_convert<T>::value) {
        convert<T>::write(out, v);
    } else if constexpr (std::is_same_v<T, bool>) {
        out += v ? "true" : "false";
    } else if constexpr (std::is_arithmetic_v<T>) {
        write_number(out, static_cast<double>(v));
    } else if constexpr (std::is_convertible_v<const T&, std::string_view>) {
        write_string(out, std::string_view(v));
    } else if constexpr (is_optional<T>::value) {
        if (v) write(out, *v);
        else out += "null";
    } else if constexpr (is_vector<T>::value) {
        out += '[';
        for (std::size_t i = 0; i < v.size(); i++) {
            if (i) out += ',';
            write(out, v[i]);
        }
        out += ']';
    } else if constexpr (is_map<T>::value) {
        out += '{';
        bool first = true;
        for (auto& [k, x] : v) {
            if (!first) out += ',';
            first = false;
            write_string(out, k);
            out += ':';
            write(out, x);
        }
        out += '}';
    }
#if UISON_REFLECT
    else if constexpr (reflectable<T>) {
        constexpr std::size_t N = count_fields<T>();
        constexpr auto names = []<std::size_t... I>(std::index_sequence<I...>) {
            return std::array<std::string_view, N>{field_name<T, I>()...};
        }(std::make_index_sequence<N>{});
        write_fields(out, field_ptrs(v), std::make_index_sequence<N>{}, names.data());
    }
#endif
    else {
        static_assert(sizeof(T) == 0, "uison: no JSON conversion for this type (specialize uison::convert<T>)");
    }
}
} // namespace detail

/// Data by path ("form.email", "todos.3.done"). Inside a handler — valid until it returns.
class Data {
    uison_data* d_;

public:
    explicit Data(uison_data* d) : d_(d) {}
    uison_data* c() const { return d_; }

    /// UISON_NONE (-1), UISON_NULL, UISON_BOOL, UISON_NUMBER, UISON_STRING, UISON_ARRAY, UISON_OBJECT.
    int type(std::string_view path) const { return uison_data_type(d_, detail::Path(path).c()); }
    bool has(std::string_view path) const { return type(path) != UISON_NONE; }
    /// A string without copying: valid until the handler returns.
    std::string_view str(std::string_view path) const { return uison_data_str(d_, detail::Path(path).c()); }
    double num(std::string_view path) const { return uison_data_f64(d_, detail::Path(path).c()); }
    /// Truthiness as in bindings: false, null, 0, "", [] are false.
    bool truthy(std::string_view path) const { return uison_data_bool(d_, detail::Path(path).c()); }
    std::size_t len(std::string_view path) const { return uison_data_len(d_, detail::Path(path).c()); }
    /// Subtree as a JSON string ("" — no such path).
    std::string json(std::string_view path) const {
        const char* s = uison_data_json(d_, detail::Path(path).c());
        return s ? s : "";
    }
    /// The value as a C++ type: string, numbers, bool, optional, vector, map<string, T>, an aggregate (C++20), Json.
    template <class T>
    T get(std::string_view path) const;
    /// No such path — `def`.
    template <class T>
    T get(std::string_view path, T def) const {
        return has(path) ? get<T>(path) : std::move(def);
    }

    template <class T>
    void set(std::string_view path, const T& v) {
        detail::Path p(path);
        if constexpr (std::is_same_v<T, bool>) {
            uison_data_set_bool(d_, p.c(), v);
        } else if constexpr (std::is_arithmetic_v<T>) {
            uison_data_set_f64(d_, p.c(), static_cast<double>(v));
        } else if constexpr (std::is_convertible_v<const T&, std::string_view>) {
            uison_data_set_str(d_, p.c(), std::string(std::string_view(v)).c_str());
        } else if constexpr (std::is_same_v<T, std::nullptr_t>) {
            uison_data_set_null(d_, p.c());
        } else {
            std::string s;
            detail::write(s, v);
            if (!uison_data_set_json(d_, p.c(), s.c_str())) throw Error(uison_last_error());
        }
    }
    /// To the end of an array (no array — one is created).
    template <class T>
    void push(std::string_view path, const T& v) {
        std::string s;
        detail::write(s, v);
        if (!uison_data_push_json(d_, detail::Path(path).c(), s.c_str())) throw Error(uison_last_error());
    }
    /// An array element ("todos.3") or an object field ("form.email"); false — nothing to remove.
    bool remove(std::string_view path) { return uison_data_remove(d_, detail::Path(path).c()); }
    /// Change a value with a function: `data.update<int>("n", [](int n) { return n + 1; })`.
    template <class T, class F>
    void update(std::string_view path, F&& f) {
        set(path, f(get<T>(path)));
    }
};

namespace detail {
template <class T>
T read(const Data& d, const std::string& path) {
    if constexpr (has_convert<T>::value) {
        return convert<T>::read(d, path);
    } else if constexpr (std::is_same_v<T, Json>) {
        return Json::raw(d.has(path) ? d.json(path) : "null");
    } else if constexpr (std::is_same_v<T, bool>) {
        return d.truthy(path);
    } else if constexpr (std::is_arithmetic_v<T>) {
        return static_cast<T>(d.num(path));
    } else if constexpr (std::is_same_v<T, std::string>) {
        return std::string(d.str(path));
    } else if constexpr (std::is_same_v<T, std::string_view>) {
        return d.str(path);
    } else if constexpr (is_optional<T>::value) {
        int t = d.type(path);
        if (t == UISON_NONE || t == UISON_NULL) return std::nullopt;
        return read<typename T::value_type>(d, path);
    } else if constexpr (is_vector<T>::value) {
        T out;
        std::size_t n = d.type(path) == UISON_ARRAY ? d.len(path) : 0;
        out.reserve(n);
        for (std::size_t i = 0; i < n; i++) out.push_back(read<typename T::value_type>(d, join(path, std::to_string(i))));
        return out;
    } else if constexpr (is_map<T>::value) {
        T out;
        if (d.type(path) != UISON_OBJECT) return out;
        std::size_t n = d.len(path);
        for (std::size_t i = 0; i < n; i++) {
            std::string k = uison_data_key(d.c(), path.c_str(), i);
            out.emplace(k, read<typename T::mapped_type>(d, join(path, k)));
        }
        return out;
    }
#if UISON_REFLECT
    else if constexpr (reflectable<T>) {
        T out{};
        constexpr std::size_t N = count_fields<T>();
        auto ptrs = field_ptrs(out);
        [&]<std::size_t... I>(std::index_sequence<I...>) {
            ((*const_cast<std::remove_const_t<std::remove_reference_t<decltype(*std::get<I>(ptrs))>>*>(std::get<I>(ptrs)) =
                  read<std::remove_cv_t<std::remove_reference_t<decltype(*std::get<I>(ptrs))>>>(d, join(path, field_name<T, I>()))),
             ...);
        }(std::make_index_sequence<N>{});
        return out;
    }
#endif
    else {
        static_assert(sizeof(T) == 0, "uison: no conversion from data for this type (specialize uison::convert<T>)");
    }
}
} // namespace detail

template <class T>
T Data::get(std::string_view path) const {
    return detail::read<T>(*this, std::string(path));
}

/// Handler context: only inside the handler.
class Cx {
    uison_cx* c_;

public:
    explicit Cx(uison_cx* c) : c_(c) {}
    uison_cx* c() const { return c_; }

    std::string_view action() const { return uison_cx_action(c_); }
    /// click, change, submit, menu, shortcut, timer, code.
    std::string_view event() const { return uison_cx_event(c_); }
    /// key of the event's node: "del-3", "list/12".
    std::string_view key() const { return uison_cx_key(c_); }
    /// The number at the end of the key ("del-3" → 3, "list/12" → 12); none — -1.
    long key_num() const {
        std::string_view k = key();
        std::size_t i = k.size();
        while (i > 0 && k[i - 1] >= '0' && k[i - 1] <= '9') i--;
        return i == k.size() ? -1 : std::strtol(std::string(k.substr(i)).c_str(), nullptr, 10);
    }
    Data data() const { return Data(uison_cx_data(c_)); }
    /// Focus a field: "name/1/0" — the text input inside a TextField with key "name".
    void focus(std::string_view key) { uison_cx_focus(c_, detail::Path(key).c()); }
    void blur() { uison_cx_blur(c_); }
    void theme(std::string_view name) { uison_cx_set_theme(c_, detail::Path(name).c()); }
    void token(std::string_view name, const Json& v) { uison_cx_set_token(c_, detail::Path(name).c(), v.c_str()); }
    void timer(std::uint64_t ms, std::string_view action) { uison_cx_timer(c_, ms, detail::Path(action).c()); }
    void copy(std::string_view text) { uison_cx_copy(c_, detail::Path(text).c()); }
    void open_url(std::string_view url) { uison_cx_open_url(c_, detail::Path(url).c()); }
    /// A screen node property as a JSON string.
    std::optional<std::string> node(std::string_view key, std::string_view prop) const {
        const char* s = uison_cx_node_get(c_, detail::Path(key).c(), detail::Path(prop).c());
        if (!s) return std::nullopt;
        return std::string(s);
    }
    bool node(std::string_view key, std::string_view prop, const Json& v) {
        return uison_cx_node_set(c_, detail::Path(key).c(), detail::Path(prop).c(), v.c_str());
    }
};

namespace detail {
/// Calling a lambda from C: an exception doesn't escape into C — the message goes to stderr.
template <class F>
void call(uison_cx* c, void* user) {
    try {
        Cx cx(c);
        auto& f = *static_cast<F*>(user);
        if constexpr (std::is_invocable_v<F&, Cx&>) f(cx);
        else f();
    } catch (const std::exception& e) {
        std::fprintf(stderr, "uison: handler '%s' threw: %s\n", uison_cx_action(c), e.what());
    } catch (...) {
        std::fprintf(stderr, "uison: handler '%s' threw\n", uison_cx_action(c));
    }
}
template <class F>
void call_derive(uison_data* d, void* user) {
    try {
        Data data(d);
        (*static_cast<F*>(user))(data);
    } catch (const std::exception& e) {
        std::fprintf(stderr, "uison: derive threw: %s\n", e.what());
    } catch (...) {
        std::fprintf(stderr, "uison: derive threw\n");
    }
}
template <class F>
void destroy(void* user) {
    delete static_cast<F*>(user);
}
} // namespace detail

class Node;

/// Handle to the app for other threads: `handle.post([](uison::Cx& cx) { … })` — runs
/// on the UI thread. Copyable, thread-safe.
class Handle {
    std::shared_ptr<uison_handle> h_;

public:
    explicit Handle(uison_handle* h) : h_(h, uison_handle_free) {}
    template <class F>
    void post(F&& f) const {
        using Fn = std::decay_t<F>;
        uison_post(h_.get(), &detail::call<Fn>, new Fn(std::forward<F>(f)), &detail::destroy<Fn>);
    }
};

/// Window size in points; 0 — default.
struct Size {
    double w = 0, h = 0;
};

/// Everything about the screen except the root, as a struct (C++20 — designated initializers, fields in order):
/// `uison::Ui ui(root, {.size = {420, 380}, .data = State{…}, .theme = "dark"})`.
struct Screen {
    Size size = {};
    Json data = nullptr;              ///< a struct (reflection, C++20), uison::obj{…}, map
    Json theme = nullptr;             ///< theme name ("light", "dark", "system") or tokens uison::obj{…}
    std::string_view components = {}; ///< your own components — JSON text { Name: { params, body } }
    std::optional<bool> safe_area = {}; ///< root inside the safe area (yes by default)
};

/// Screen, handlers, data. Owns the `uison_ui*` until `run`.
class Ui {
    uison_ui* u_ = nullptr;

    static uison_ui* check(uison_ui* u) {
        if (!u) throw Error(uison_last_error() ? uison_last_error() : "uison: error");
        return u;
    }

public:
    /// Screen from JSON (`R"( … )"`); a parse error throws `uison::Error` with line:col.
    explicit Ui(std::string_view json) : u_(check(uison_ui_new(detail::Path(json).c()))) {}
    /// Screen from the builder (`uison::column()…`): its lambdas become handlers;
    /// `screen` — the rest: {{"size", uison::arr{520, 620}}, {"data", uison::obj{…}}}.
    explicit Ui(const Node& root, std::initializer_list<std::pair<std::string_view, Json>> screen = {});
    /// The same with options as a struct: `Ui(root, {.size = {420, 380}, .data = State{…}})`.
    Ui(const Node& root, const Screen& screen);
    /// Screen from a file (next to the program or in the current folder).
    static Ui load(std::string_view path) { return Ui(check(uison_ui_load(detail::Path(path).c()))); }
    explicit Ui(uison_ui* u) : u_(u) {}
    Ui(Ui&& o) noexcept : u_(std::exchange(o.u_, nullptr)) {}
    Ui& operator=(Ui&& o) noexcept {
        if (this != &o) {
            uison_ui_free(u_);
            u_ = std::exchange(o.u_, nullptr);
        }
        return *this;
    }
    Ui(const Ui&) = delete;
    Ui& operator=(const Ui&) = delete;
    ~Ui() { uison_ui_free(u_); }
    uison_ui* c() const { return u_; }

    /// Handler: `[](uison::Cx& cx) { … }` or `[] { … }`.
    template <class F>
    Ui& on(std::string_view action, F&& f) {
        using Fn = std::decay_t<F>;
        uison_ui_on(u_, detail::Path(action).c(), &detail::call<Fn>, new Fn(std::forward<F>(f)), &detail::destroy<Fn>);
        return *this;
    }
    /// Derived data: `[](uison::Data& d) { … }` — right away and after every change.
    template <class F>
    Ui& derive(F&& f) {
        using Fn = std::decay_t<F>;
        uison_ui_derive(u_, &detail::call_derive<Fn>, new Fn(std::forward<F>(f)), &detail::destroy<Fn>);
        return *this;
    }
    Ui& shortcut(std::string_view combo, std::string_view action) {
        if (!uison_ui_shortcut(u_, detail::Path(combo).c(), detail::Path(action).c())) throw Error(uison_last_error());
        return *this;
    }
    Ui& theme(std::string_view name) {
        uison_ui_set_theme(u_, detail::Path(name).c());
        return *this;
    }
    Ui& components(std::string_view json) {
        if (!uison_ui_add_components(u_, detail::Path(json).c())) throw Error(uison_last_error());
        return *this;
    }
    Ui& components_file(std::string_view path) {
        if (!uison_ui_load_components(u_, detail::Path(path).c())) throw Error(uison_last_error());
        return *this;
    }
    /// Image from PNG/JPEG bytes → the source "mem:name".
    std::string image(std::string_view name, const void* bytes, std::size_t len) {
        return uison_ui_image(u_, detail::Path(name).c(), static_cast<const std::uint8_t*>(bytes), len);
    }
    template <class Bytes>
    std::string image(std::string_view name, const Bytes& b) {
        return image(name, std::data(b), std::size(b));
    }
    /// Image from a file (next to the program, then the current folder) → "mem:name"; no file — uison::Error.
    std::string image_file(std::string_view name, std::string_view path) {
        const char* s = uison_ui_image_file(u_, detail::Path(name).c(), detail::Path(path).c());
        if (!s) throw Error(uison_last_error());
        return s;
    }
    Data data() { return Data(uison_ui_data(u_)); }
    /// Handle for worker threads (get it before `run`).
    Handle handle() const { return Handle(uison_ui_handle(u_)); }
    void run_action(std::string_view action) { uison_ui_run_action(u_, detail::Path(action).c()); }
    /// A JSON message, as from outside: `{"action":…,"key":…}` or `{"data":{…}}`; `false` — not a known message.
    bool apply(std::string_view msg) { return uison_ui_apply(u_, detail::Path(msg).c()); }
    /// Open the window; the `Ui` is empty afterwards.
    int run(std::string_view title) { return uison_run(std::exchange(u_, nullptr), detail::Path(title).c()); }
};

// ---------------------------------------------------------------- screen builder

/// A screen node as a method chain (like the Rust DSL): `uison::row().gap(8).children({ uison::text("Tasks").bold(), … })`.
/// `.prop("anything", value)` — for any property and your own components; lambdas — `.on_click([&] { … })`.
/// Options of Select, RadioGroup, Tabs: `uison::choices({{"all", "All"}, {"done", "Done"}})`
/// → `[{"value":"all","label":"All"}, …]`.
inline Json choices(std::initializer_list<std::pair<std::string_view, std::string_view>> items) {
    std::string s = "[";
    bool first = true;
    for (auto& [v, l] : items) {
        if (!first) s += ',';
        first = false;
        s += "{\"value\":";
        detail::write_string(s, v);
        s += ",\"label\":";
        detail::write_string(s, l);
        s += '}';
    }
    return Json::raw(s + ']');
}

class Node {
    using Handlers = std::vector<std::pair<std::string, std::function<void(Cx&)>>>;
    std::string kind_;
    std::vector<std::pair<std::string, Json>> props_;
    std::vector<std::pair<std::string, Node>> nodes_; // node-valued properties: child, leading, do…
    std::vector<Node> children_;
    std::vector<std::pair<std::string, std::string>> on_;
    Handlers handlers_;

    static std::string next_action() {
        static int n = 0;
        return "~h" + std::to_string(++n);
    }
    template <class F>
    static std::function<void(Cx&)> wrap(F&& f) {
        if constexpr (std::is_invocable_v<F&, Cx&>) return std::forward<F>(f);
        else return [g = std::forward<F>(f)](Cx&) mutable { g(); };
    }

public:
    explicit Node(std::string kind) : kind_(std::move(kind)) {}
    const std::string& kind() const { return kind_; }

    Node& prop(std::string_view name, const Json& v) {
        for (auto& p : props_)
            if (p.first == name) {
                p.second = v;
                return *this;
            }
        props_.emplace_back(std::string(name), v);
        return *this;
    }
    /// A node-valued property (its lambdas become handlers too).
    Node& prop(std::string_view name, Node n) {
        for (auto& p : nodes_)
            if (p.first == name) {
                p.second = std::move(n);
                return *this;
            }
        nodes_.emplace_back(std::string(name), std::move(n));
        return *this;
    }

    // Layout
    Node& key(std::string_view v) { return prop("key", v); }
    Node& w(const Json& v) { return prop("w", v); }
    Node& h(const Json& v) { return prop("h", v); }
    Node& fill_w() { return prop("w", "fill"); }
    Node& fill_h() { return prop("h", "fill"); }
    Node& min_w(double v) { return prop("minW", v); }
    Node& max_w(double v) { return prop("maxW", v); }
    Node& flex(double v = 1) { return prop("flex", v); }
    Node& padding(const Json& v) { return prop("padding", v); }
    Node& margin(const Json& v) { return prop("margin", v); }
    Node& gap(const Json& v) { return prop("gap", v); }
    /// Main axis: start, center, end, between, around, evenly.
    Node& main(std::string_view v) { return prop("main", v); }
    /// Cross axis: start, center, end, stretch.
    Node& cross(std::string_view v) { return prop("cross", v); }
    Node& align(std::string_view v) { return prop("align", v); }
    Node& visible(const Json& v) { return prop("visible", v); }
    Node& disabled(const Json& v) { return prop("disabled", v); }
    Node& tooltip(const Json& v) { return prop("tooltip", v); }
    Node& context_menu(const Json& items) { return prop("contextMenu", items); }
    // Appearance
    Node& fill(const Json& v) { return prop("fill", v); }
    Node& border(const Json& color, double width = 1) { return prop("stroke", color).prop("width", width); }
    Node& radius(const Json& v) { return prop("radius", v); }
    Node& color(const Json& v) { return prop("color", v); }
    Node& size(const Json& v) { return prop("size", v); }
    Node& bold(bool v = true) { return prop("bold", v); }
    Node& max_lines(int n) { return prop("maxLines", n); }
    /// Text alignment: left, center, right.
    Node& text_align(std::string_view v) { return prop("align", v); }
    // Library elements
    /// Bind the value to a data path: Checkbox/Switch — checked, the rest — value.
    Node& bind(std::string_view path) {
        std::string b = "@" + std::string(path);
        return prop(kind_ == "Checkbox" || kind_ == "Switch" ? "checked" : "value", b);
    }
    Node& text(const Json& v) { return prop("text", v); }
    Node& label(const Json& v) { return prop("label", v); }
    Node& placeholder(const Json& v) { return prop("placeholder", v); }
    Node& error(const Json& v) { return prop("error", v); }
    Node& password(bool v = true) { return prop("password", v); }
    Node& icon(const Json& v) { return prop("icon", v); }
    Node& variant(std::string_view v) { return prop("variant", v); }
    Node& src(const Json& v) { return prop("src", v); }
    Node& title(const Json& v) { return prop("title", v); }
    Node& subtitle(const Json& v) { return prop("subtitle", v); }
    Node& range(double min, double max, double step = 0) { return prop("min", min).prop("max", max).prop("step", step); }
    /// Options of Select/RadioGroup: {{"value", "Label"}, …}.
    Node& options(std::initializer_list<std::pair<std::string_view, std::string_view>> items) { return prop("options", choices(items)); }
    /// Tabs: {{"all", "All"}, …}.
    Node& tabs(std::initializer_list<std::pair<std::string_view, std::string_view>> items) { return prop("tabs", choices(items)); }
    Node& leading(Node n) { return prop("leading", std::move(n)); }
    Node& trailing(Node n) { return prop("trailing", std::move(n)); }
    // Children
    Node& child(Node n) { return prop("child", std::move(n)); }
    Node& children(std::initializer_list<Node> ns) {
        for (auto& n : ns) children_.push_back(n);
        return *this;
    }
    Node& children(std::vector<Node> ns) {
        for (auto& n : ns) children_.push_back(std::move(n));
        return *this;
    }
    // Events: an action name or a lambda (the name is made up automatically)
    Node& on(std::string_view event, std::string_view action) {
        on_.emplace_back(std::string(event), std::string(action));
        return *this;
    }
    template <class F, std::enable_if_t<!std::is_convertible_v<F, std::string_view>, int> = 0>
    Node& on(std::string_view event, F&& f) {
        std::string a = next_action();
        handlers_.emplace_back(a, wrap(std::forward<F>(f)));
        return on(event, a);
    }
    template <class F>
    Node& on_click(F&& f) { return on("click", std::forward<F>(f)); }
    template <class F>
    Node& on_change(F&& f) { return on("change", std::forward<F>(f)); }
    template <class F>
    Node& on_submit(F&& f) { return on("submit", std::forward<F>(f)); }

    /// The node as JSON: { Kind: { props…, on: {…}, children: […] } }.
    Json json() const {
        std::string s = "{";
        detail::write_string(s, kind_);
        s += ":{";
        bool first = true;
        auto field = [&](std::string_view k, const std::string& v) {
            if (!first) s += ',';
            first = false;
            detail::write_string(s, k);
            s += ':';
            s += v;
        };
        for (auto& [k, v] : props_) field(k, v.str());
        for (auto& [k, n] : nodes_) field(k, n.json().str());
        if (!on_.empty()) {
            std::string o = "{";
            for (std::size_t i = 0; i < on_.size(); i++) {
                if (i) o += ',';
                detail::write_string(o, on_[i].first);
                o += ':';
                detail::write_string(o, on_[i].second);
            }
            field("on", o + '}');
        }
        if (!children_.empty()) {
            std::string c = "[";
            for (std::size_t i = 0; i < children_.size(); i++) {
                if (i) c += ',';
                c += children_[i].json().str();
            }
            field("children", c + ']');
        }
        return Json::raw(s + "}}");
    }
    operator Json() const { return json(); }

    /// Lambdas of this node and all nested ones — to register in the Ui.
    void collect(Handlers& out) const {
        for (auto& h : handlers_) out.push_back(h);
        for (auto& [k, n] : nodes_) n.collect(out);
        for (auto& c : children_) c.collect(out);
    }
};

inline Node node(std::string kind) { return Node(std::move(kind)); }
inline Node column() { return Node("Column"); }
inline Node row() { return Node("Row"); }
inline Node stack() { return Node("Stack"); }
inline Node wrap() { return Node("Wrap"); }
inline Node flow() { return Node("Flow"); } // inline flow: text children form one paragraph
inline Node container() { return Node("Box"); }
inline Node center(Node child) { return Node("Center").child(std::move(child)); }
inline Node scroll(Node child) { return Node("Scroll").child(std::move(child)); }
inline Node spacer() { return Node("Spacer"); }
inline Node text(const Json& t) { return Node("Text").text(t); }
inline Node button(const Json& t) { return Node("Button").text(t); }
inline Node icon_button(const Json& icon) { return Node("IconButton").icon(icon); }
inline Node icon(const Json& name) { return Node("Icon").prop("name", name); }
inline Node checkbox(const Json& label = nullptr) { return Node("Checkbox").label(label); }
inline Node toggle(const Json& label = nullptr) { return Node("Switch").label(label); }
inline Node radio_group() { return Node("RadioGroup"); }
inline Node select() { return Node("Select"); }
inline Node tabs() { return Node("Tabs"); }
inline Node text_field() { return Node("TextField"); }
inline Node slider() { return Node("Slider"); }
inline Node progress(const Json& value) { return Node("ProgressBar").prop("value", value); }
inline Node divider() { return Node("Divider"); }
inline Node badge(const Json& t) { return Node("Badge").text(t); }
inline Node avatar(const Json& name) { return Node("Avatar").prop("name", name); }
inline Node list_tile(const Json& title) { return Node("ListTile").title(title); }
inline Node image(const Json& src) { return Node("Image").src(src); }
inline Node link(const Json& text, const Json& href) { return Node("Link").text(text).prop("href", href); }
inline Node card(std::initializer_list<Node> children) { return Node("Card").children(children); }
/// For: each element of the array `each` ("todos") under the name `as` — from the template `item`.
inline Node for_each(std::string_view each, std::string_view as, Node item) {
    return Node("For").prop("each", "@" + std::string(each)).prop("as", as).prop("do", std::move(item));
}
/// List: a virtualized list (only visible rows are drawn), row height — `item_height`.
inline Node list(std::string_view items, std::string_view as, double item_height, Node item) {
    return Node("List").prop("items", "@" + std::string(items)).prop("as", as).prop("itemHeight", item_height).prop("item", std::move(item));
}

#if defined(_MSC_VER)
#define UISON_NOINLINE __declspec(noinline)
#else
#define UISON_NOINLINE __attribute__((noinline))
#endif

/// A property value in widgets (`uison_widgets.hpp`): unset, JSON (`20`, `"@name"`, `{12, 16}`) or a node.
/// Inside — a single pointer: a widget has dozens of fields, unset ones cost nothing at the call site.
class Prop {
    struct Value {
        std::optional<Json> json;
        std::optional<Node> node;
    };
    Value* v_ = nullptr;
    static UISON_NOINLINE void destroy(Value* v) { delete v; }

public:
    Prop() = default;
    Prop(Node n) : v_(new Value{std::nullopt, std::move(n)}) {}
    template <class T, std::enable_if_t<!std::is_same_v<std::decay_t<T>, Prop> && !std::is_same_v<std::decay_t<T>, Node> &&
                                            std::is_constructible_v<Json, T&&>,
                                        int> = 0>
    Prop(T&& v) : v_(new Value{Json(std::forward<T>(v)), std::nullopt}) {}
    /// Array: `{12, 16}`, `{"a", "b"}`.
    Prop(std::initializer_list<Json> items) : v_(new Value{Json(arr(items)), std::nullopt}) {}
    Prop(const Prop& o) : v_(o.v_ ? new Value(*o.v_) : nullptr) {}
    Prop(Prop&& o) noexcept : v_(std::exchange(o.v_, nullptr)) {}
    Prop& operator=(Prop o) noexcept {
        std::swap(v_, o.v_);
        return *this;
    }
    ~Prop() {
        if (v_) destroy(v_);
    }
    bool empty() const { return !v_; }
    /// Write into the node under the name `name`, if set.
    void apply(Node& n, const char* name) const {
        if (v_) put(n, name, *v_);
    }

private:
    static UISON_NOINLINE void put(Node& n, const char* name, const Value& v) {
        if (v.json) n.prop(name, *v.json);
        else n.prop(name, *v.node);
    }
};

/// Event handler in widgets: an action name (`"save"`) or a lambda (`[](uison::Cx& cx) {…}`, `[] {…}`).
class Action {
    struct Value {
        std::string name;
        std::function<void(Cx&)> f;
    };
    Value* v_ = nullptr;
    static UISON_NOINLINE void destroy(Value* v) { delete v; }

public:
    Action() = default;
    Action(const char* name) : v_(new Value{name ? name : "", {}}) {}
    Action(std::string name) : v_(new Value{std::move(name), {}}) {}
    Action(std::string_view name) : v_(new Value{std::string(name), {}}) {}
    template <class F, std::enable_if_t<std::is_invocable_v<F&, Cx&> || std::is_invocable_v<F&>, int> = 0>
    Action(F f) : v_(new Value) {
        if constexpr (std::is_invocable_v<F&, Cx&>) v_->f = std::move(f);
        else v_->f = [g = std::move(f)](Cx&) mutable { g(); };
    }
    Action(const Action& o) : v_(o.v_ ? new Value(*o.v_) : nullptr) {}
    Action(Action&& o) noexcept : v_(std::exchange(o.v_, nullptr)) {}
    Action& operator=(Action o) noexcept {
        std::swap(v_, o.v_);
        return *this;
    }
    ~Action() {
        if (v_) destroy(v_);
    }
    void apply(Node& n, const char* event) const {
        if (v_) put(n, event, *v_);
    }

private:
    static UISON_NOINLINE void put(Node& n, const char* event, const Value& v) {
        if (v.f) n.on(event, v.f);
        else if (!v.name.empty()) n.on(event, v.name);
    }
};

namespace detail {
/// Widget: node `kind` and all set property values from the struct's field table.
template <class P, std::size_t N>
Node widget(const char* kind, const P& p, Prop P::* const (&fields)[N], const char* const (&names)[N]) {
    Node n(kind);
    for (std::size_t i = 0; i < N; i++) (p.*fields[i]).apply(n, names[i]);
    return n;
}
} // namespace detail

/// Ui from the builder: `screen` — the rest of the screen description ({"size", uison::arr{520, 620}}, {"data", …}, {"theme", …}).
namespace detail {
inline std::string screen_json(const Node& root, const Screen& o) {
    std::string s = "{\"root\":" + root.json().str();
    if (o.size.w > 0 && o.size.h > 0) {
        s += ",\"size\":[";
        write_number(s, o.size.w);
        s += ',';
        write_number(s, o.size.h);
        s += ']';
    }
    if (o.data.str() != "null") s += ",\"data\":" + o.data.str();
    if (o.theme.str() != "null") s += ",\"theme\":" + o.theme.str();
    if (!o.components.empty()) s += ",\"components\":" + std::string(o.components);
    if (o.safe_area) s += *o.safe_area ? ",\"safeArea\":true" : ",\"safeArea\":false";
    return s + '}';
}
} // namespace detail

inline Ui::Ui(const Node& root, const Screen& screen) : Ui(std::string_view(detail::screen_json(root, screen))) {
    std::vector<std::pair<std::string, std::function<void(Cx&)>>> hs;
    root.collect(hs);
    for (auto& [a, f] : hs) on(a, std::move(f));
}

inline Ui::Ui(const Node& root, std::initializer_list<std::pair<std::string_view, Json>> screen)
    : Ui(std::string_view([&] {
          std::string s = "{\"root\":" + root.json().str();
          for (auto& [k, v] : screen) {
              s += ',';
              detail::write_string(s, k);
              s += ':';
              s += v.str();
          }
          return s + '}';
      }())) {
    std::vector<std::pair<std::string, std::function<void(Cx&)>>> hs;
    root.collect(hs);
    for (auto& [a, f] : hs) on(a, std::move(f));
}

// ---------------------------------------------------------------- low level: your own frame

/// Platform access during `frame`/`on_event`: text metrics, clipboard, links.
class Host {
    uison_host* h_;

public:
    explicit Host(uison_host* h) : h_(h) {}
    struct Metrics {
        double width = 0, ascent = 0, descent = 0;
    };
    /// Width and line heights of a text string: system font (`family` — a name or monospace/serif/sans-serif).
    Metrics measure(std::string_view text, double size, bool bold = false, std::string_view family = {}) const {
        Metrics m;
        std::string f(family);
        uison_host_measure(h_, detail::Path(text).c(), size, bold, f.empty() ? nullptr : f.c_str(), &m.width, &m.ascent,
                           &m.descent);
        return m;
    }
    void copy(std::string_view text) const { uison_host_copy(h_, detail::Path(text).c()); }
    void request_paste() const { uison_host_request_paste(h_); } ///< the answer is a "paste" event
    bool open_url(std::string_view url) const { return uison_host_open_url(h_, detail::Path(url).c()); }
};

/// Event: type — down, up, move, leave, scroll, key, resize, appearance, tick, paste, wake…
struct Event {
    std::string type;
    double x = 0, y = 0;   ///< mouse, touch
    double dx = 0, dy = 0; ///< scroll
    double w = 0, h = 0;   ///< resize
    std::string button;    ///< left, right, other, touch
    std::string code;      ///< key: char, enter, backspace, left…
    std::string text;      ///< key (character), paste
    bool shift = false, ctrl = false, alt = false, cmd = false;
    bool dark = false; ///< appearance
};

namespace detail {
// Flat object {"type":"down","x":1.5,…}: strings, numbers (without locale), true/false/null.
inline Event parse_event(const char* s) {
    Event e;
    auto ws = [&] {
        while (*s == ' ' || *s == '\n' || *s == '\t' || *s == '\r') s++;
    };
    auto str = [&] {
        std::string out;
        if (*s != '"') return out;
        s++;
        while (*s && *s != '"') {
            if (*s == '\\' && s[1]) {
                s++;
                char c = *s++;
                if (c == 'n') out += '\n';
                else if (c == 't') out += '\t';
                else if (c == 'r') out += '\r';
                else if (c == 'u') {
                    unsigned cp = static_cast<unsigned>(std::strtoul(std::string(s, 4).c_str(), nullptr, 16));
                    s += 4;
                    if (cp >= 0xD800 && cp < 0xDC00 && s[0] == '\\' && s[1] == 'u') {
                        unsigned lo = static_cast<unsigned>(std::strtoul(std::string(s + 2, 4).c_str(), nullptr, 16));
                        s += 6;
                        cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                    }
                    if (cp < 0x80) out += static_cast<char>(cp);
                    else if (cp < 0x800) out += static_cast<char>(0xC0 | (cp >> 6)), out += static_cast<char>(0x80 | (cp & 63));
                    else if (cp < 0x10000)
                        out += static_cast<char>(0xE0 | (cp >> 12)), out += static_cast<char>(0x80 | ((cp >> 6) & 63)),
                            out += static_cast<char>(0x80 | (cp & 63));
                    else
                        out += static_cast<char>(0xF0 | (cp >> 18)), out += static_cast<char>(0x80 | ((cp >> 12) & 63)),
                            out += static_cast<char>(0x80 | ((cp >> 6) & 63)), out += static_cast<char>(0x80 | (cp & 63));
                } else out += c;
            } else out += *s++;
        }
        if (*s == '"') s++;
        return out;
    };
    auto num = [&] {
        // Digits as an integer, one division by an exact power of ten: correctly rounded for the
        // numbers events carry (up to 15 digits), unlike adding digit × 0.1… (off by an ulp).
        double sign = 1, mant = 0;
        int exp10 = 0;
        if (*s == '-') sign = -1, s++;
        for (; *s >= '0' && *s <= '9'; s++) mant = mant * 10 + (*s - '0');
        if (*s == '.')
            for (s++; *s >= '0' && *s <= '9'; s++) mant = mant * 10 + (*s - '0'), exp10--;
        if (*s == 'e' || *s == 'E') {
            s++;
            int es = 1, ex = 0;
            if (*s == '-' || *s == '+') es = *s++ == '-' ? -1 : 1;
            while (*s >= '0' && *s <= '9') ex = ex * 10 + (*s++ - '0');
            exp10 += es * ex;
        }
        double p = std::pow(10.0, exp10 < 0 ? -exp10 : exp10);
        return sign * (exp10 < 0 ? mant / p : mant * p);
    };
    ws();
    if (*s != '{') return e;
    s++;
    for (;;) {
        ws();
        if (*s != '"') break;
        std::string k = str();
        ws();
        if (*s == ':') s++;
        ws();
        std::string sv;
        double nv = 0;
        bool bv = false;
        if (*s == '"') sv = str();
        else if (*s == 't') bv = true, s += 4;
        else if (*s == 'f') s += 5;
        else if (*s == 'n') s += 4;
        else nv = num();
        if (k == "type") e.type = sv;
        else if (k == "x") e.x = nv;
        else if (k == "y") e.y = nv;
        else if (k == "dx") e.dx = nv;
        else if (k == "dy") e.dy = nv;
        else if (k == "w") e.w = nv;
        else if (k == "h") e.h = nv;
        else if (k == "button") e.button = sv;
        else if (k == "code") e.code = sv;
        else if (k == "text") e.text = sv;
        else if (k == "shift") e.shift = bv;
        else if (k == "ctrl") e.ctrl = bv;
        else if (k == "alt") e.alt = bv;
        else if (k == "cmd") e.cmd = bv;
        else if (k == "dark") e.dark = bv;
        ws();
        if (*s == ',') s++;
    }
    return e;
}
} // namespace detail

/// Fill and stroke of a shape: `{.fill = "#FFFFFF", .stroke = "#D2D2D7", .width = 1, .radius = 10}`.
struct Paint {
    std::string_view fill = {}, stroke = {};
    double width = 0, radius = 0;
};
/// Text: `at` — top-left corner of the line.
struct TextStyle {
    double size = 13;
    std::string_view color = {};
    bool bold = false, italic = false;
    std::string_view font = {}; ///< a name or monospace, serif, sans-serif
};
struct LineStyle {
    std::string_view color = {};
    double width = 1;
    bool round = true; ///< rounded caps
};

/// A frame of primitives without hand-written JSON: `FrameWriter f(400, 300, "#F5F5F7"); f.rect(…).text(…); return f.str();`
class FrameWriter {
    std::string s_;
    std::vector<bool> first_{true};

    void item(const char* geo, std::initializer_list<double> v) {
        s_ += first_.back() ? "{\"" : ",{\"";
        first_.back() = false;
        s_ += geo;
        s_ += "\":[";
        bool f = true;
        for (double x : v) {
            if (!f) s_ += ',';
            f = false;
            detail::write_number(s_, x);
        }
        s_ += ']';
    }
    void key(const char* k, std::string_view v) {
        if (v.empty()) return;
        s_ += ",\"";
        s_ += k;
        s_ += "\":";
        detail::write_string(s_, v);
    }
    void key(const char* k, double v) {
        s_ += ",\"";
        s_ += k;
        s_ += "\":";
        detail::write_number(s_, v);
    }
    void paint(const Paint& p) {
        key("fill", p.fill);
        key("stroke", p.stroke);
        if (p.width > 0) key("width", p.width);
        if (p.radius > 0) key("radius", p.radius);
    }

public:
    FrameWriter(double w, double h, std::string_view background = {}) {
        s_ = "{\"size\":[";
        detail::write_number(s_, w);
        s_ += ',';
        detail::write_number(s_, h);
        s_ += ']';
        key("background", background);
        s_ += ",\"items\":[";
    }
    FrameWriter& rect(double x, double y, double w, double h, const Paint& p = {}) {
        item("rect", {x, y, w, h});
        paint(p);
        s_ += '}';
        return *this;
    }
    FrameWriter& circle(double cx, double cy, double r, const Paint& p = {}) {
        item("circle", {cx, cy, r});
        paint(p);
        s_ += '}';
        return *this;
    }
    FrameWriter& tri(double x1, double y1, double x2, double y2, double x3, double y3, const Paint& p = {}) {
        item("tri", {x1, y1, x2, y2, x3, y3});
        paint(p);
        s_ += '}';
        return *this;
    }
    /// Closed polygon: `count` points, coordinates in a row x1, y1, x2, y2, … (at least three).
    FrameWriter& poly(const double* xy, std::size_t count, const Paint& p = {}) {
        s_ += first_.back() ? "{\"poly\":[" : ",{\"poly\":[";
        first_.back() = false;
        for (std::size_t i = 0; i < 2 * count; i++) {
            if (i) s_ += ',';
            detail::write_number(s_, xy[i]);
        }
        s_ += ']';
        paint(p);
        s_ += '}';
        return *this;
    }
    FrameWriter& line(double x1, double y1, double x2, double y2, const LineStyle& l = {}) {
        item("line", {x1, y1, x2, y2});
        key("color", l.color);
        key("width", l.width);
        if (!l.round) key("cap", std::string_view("butt"));
        s_ += '}';
        return *this;
    }
    FrameWriter& text(std::string_view t, double x, double y, const TextStyle& st = {}) {
        s_ += first_.back() ? "{\"text\":" : ",{\"text\":";
        first_.back() = false;
        detail::write_string(s_, t);
        s_ += ",\"at\":[";
        detail::write_number(s_, x);
        s_ += ',';
        detail::write_number(s_, y);
        s_ += ']';
        key("size", st.size);
        key("color", st.color);
        if (st.bold) s_ += ",\"bold\":true";
        if (st.italic) s_ += ",\"italic\":true";
        key("font", st.font);
        s_ += '}';
        return *this;
    }
    /// Invisible mouse area: the event arrives with `hit` = id; cursor — arrow, pointer, text.
    FrameWriter& hit(double x, double y, double w, double h, std::string_view id, std::string_view cursor = "pointer") {
        item("hit", {x, y, w, h});
        key("id", id);
        key("cursor", cursor);
        s_ += '}';
        return *this;
    }
    /// Image: src — "mem:name" (Ui::image) or a path; fit — cover, contain, fill.
    FrameWriter& image(std::string_view src, double x, double y, double w, double h, std::string_view fit = {},
                       double radius = 0) {
        item("image", {x, y, w, h});
        key("src", src);
        key("fit", fit);
        if (radius > 0) key("radius", radius);
        s_ += '}';
        return *this;
    }
    /// Clipping: everything up to `end_clip()` is inside the rectangle.
    FrameWriter& clip(double x, double y, double w, double h) {
        item("clip", {x, y, w, h});
        s_ += ",\"items\":[";
        first_.push_back(true);
        return *this;
    }
    FrameWriter& end_clip() {
        if (first_.size() > 1) {
            first_.pop_back();
            s_ += "]}";
        }
        return *this;
    }
    /// The finished frame (unclosed clips are closed).
    std::string str() const {
        std::string s = s_;
        for (std::size_t i = 1; i < first_.size(); i++) s += "]}";
        return s + "]}";
    }
};

/// Your own app without JSON screens: the frame of primitives and the events are up to you.
class App {
public:
    virtual ~App() = default;
    /// The whole frame (`FrameWriter(…).….str()`).
    virtual std::string frame(Host& host) = 0;
    /// Event; `hit` — id of the hit area under the cursor, or empty. true — the frame changed (frame is called again).
    virtual bool on_event(const Event& e, std::string_view hit, Host& host) {
        (void)e, (void)hit, (void)host;
        return false;
    }
    /// In how many µs to send a "tick"; -1 — not needed (animations: 16000).
    virtual long long wants_tick_us() { return -1; }
};

namespace detail {
struct AppAdapter {
    App* app;
    std::string last;
    static const char* frame(void* u, uison_host* h) {
        auto* a = static_cast<AppAdapter*>(u);
        Host host(h);
        a->last = a->app->frame(host);
        return a->last.c_str();
    }
    static const char* on_event(void* u, const char* ev, const char* hit, uison_host* h) {
        auto* a = static_cast<AppAdapter*>(u);
        Host host(h);
        if (!a->app->on_event(parse_event(ev), hit ? std::string_view(hit) : std::string_view(), host)) return nullptr;
        a->last = a->app->frame(host);
        return a->last.c_str();
    }
    static int64_t tick(void* u) { return static_cast<AppAdapter*>(u)->app->wants_tick_us(); }
    static void drop(void* u) { delete static_cast<AppAdapter*>(u); }
};
} // namespace detail

/// Program icon (PNG bytes) for the running window — call before run(); see uison_set_icon.
inline void set_icon(const void* png, size_t len) { uison_set_icon(static_cast<const uint8_t*>(png), len); }

/// Open a window with your own app; `app` must live until exit (usually a variable in main).
inline int run(App& app, std::string_view title) {
    auto* a = new detail::AppAdapter{&app, {}};
    uison_app c{sizeof(uison_app), a, &detail::AppAdapter::frame, &detail::AppAdapter::on_event, &detail::AppAdapter::tick,
                &detail::AppAdapter::drop};
    return uison_run_app(&c, detail::Path(title).c());
}

} // namespace uison

#endif // UISON_HPP
