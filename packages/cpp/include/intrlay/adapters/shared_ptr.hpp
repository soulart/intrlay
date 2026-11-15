#pragma once
#include "../adapter.hpp"
#include <memory>

namespace intrlay {

/// Including this header opts shared_ptr<T> into independent value serialization.
/// Every occurrence copies its pointee into a separate Box wire region. Identity,
/// reference counts, weak references, and cycles are not part of the representation.
/// Like a unique_ptr<T> source, the pointer must be non-null; optional absence is
/// represented by std::optional. Contents must stay stable during both encoding passes.
/// Array pointers are excluded because shared_ptr<T[]> does not carry a length.
template<class T>
    requires (!std::is_array_v<T> && !std::is_void_v<T>)
struct Adapter<std::shared_ptr<T>> {
    using schema_type = std::unique_ptr<T>;

    static const std::shared_ptr<T>& source(const std::shared_ptr<T>& value) noexcept {
        return value;
    }
};

} // namespace intrlay
