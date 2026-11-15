#include <intrlay/annotations.hpp>
#include <intrlay/adapters/shared_ptr.hpp>
#include <cstdint>
#include <string>
#include <variant>
#include <tuple>
#include <optional>
#include <vector>

namespace codec_fixture {
struct INTRLAY() TupleOwners {
    std::tuple<std::uint32_t, std::string> pair;
    std::tuple<std::tuple<std::uint32_t>, std::optional<std::tuple<std::string, bool>>> nested;
    std::tuple<> empty;
    std::shared_ptr<std::uint32_t> first;
    std::shared_ptr<const std::uint32_t> second;
    std::shared_ptr<std::vector<std::uint32_t>> values;
};

using TupleRoot INTRLAY() = std::tuple<std::uint32_t, std::string>;

struct ValueRecord {
    std::uint32_t value;
    std::string label;
};

struct INTRLAY() CodecCases {
    float raw[3];
    std::variant<std::monostate, ValueRecord> choice;
};

struct CharacterTraits : std::char_traits<char> {};

struct INTRLAY() Text {
    std::basic_string<char, CharacterTraits> value;
};

struct INTRLAY() Repeated {
    std::variant<ValueRecord, ValueRecord> choice;
};

// Names are resolved even when the alias follows an earlier use of the type.
using Endpoints INTRLAY(
    alternative(0, name = "Start"),
    alternative(1, name = "End")
) = std::variant<ValueRecord, ValueRecord>;
}
