#include <intrlay/annotations.hpp>
#include <cstdint>
#include <vector>

namespace annotation_fixture {
struct alignas(16) INTRLAY(transparent) CppId {
    std::uint32_t value;
};
struct INTRLAY() RenamedField {
    INTRLAY(name="exported_value", typescript.name="literalValue", python.name="literal_value")
    std::uint32_t native_value;
    CppId identifier;
};
enum class INTRLAY() CppChoice : std::uint32_t {
    Original INTRLAY(name="Common", typescript.name="Specific") = 4
};
struct INTRLAY(view.generate_setters = false) ConfiguredViews {
    INTRLAY(view.generate_setters = true,
            view.elements.generate_setters = false,
            typescript.view.elements.generate_byte_range_accessors = true)
    std::vector<RenamedField> items;
};
}
