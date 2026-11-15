#include "alpha.hpp"
#include "beta.hpp"
#include "bindings.hpp"
#include "arrays.hpp"
#include "contracts.hpp"
#include "cpp-annotations.hpp"
#include "cpp-codecs.hpp"
#include "shared-alpha.hpp"
#include "shared-beta.hpp"
#include "custom-alpha.hpp"
#include "custom-beta.hpp"
#include "../../crates/intrlay-frontend-cpp/tests/fixtures/annotations.hpp"
#include "../../crates/intrlay-frontend-cpp/tests/fixtures/codec_cases.hpp"
#include <cassert>

template<class V> concept CanSetValue = requires (V value) { value.set_value(std::uint32_t{1}); };
template<class V> concept CanReadValue = requires (V value) { value.value(); };
template<class View, class Source>
concept CanCreateFrom = requires (const Source& value) { View::create(value); };
template<class View, class Source>
concept CanReplaceFrom = requires (View view, const Source& value) { view.replace(value); };

int main() {
    using namespace intrlay_generated::alpha;
    using namespace intrlay_generated::beta;
    using namespace intrlay_generated::regression;
    using namespace intrlay_generated::arrays;
    using namespace intrlay_generated::contracts;
    using namespace intrlay_generated::annotations;
    namespace left = intrlay_generated::shared_alpha;
    namespace right = intrlay_generated::shared_beta;
    static_assert(!std::is_same_v<left::SharedInput<>, right::SharedInput<>>);
    auto shared_alpha = left::SharedAlphaView::create(left::SharedAlphaInput<>{{11}});
    auto shared_beta = right::SharedBetaView::create(right::SharedBetaInput<>{{22}});
    assert(shared_alpha.view_unchecked().shared().value() == 11);
    assert(shared_beta.view_unchecked().shared().value() == 22);
    auto custom_alpha = fixture::left::SharedView::create(fixture::left::SharedInput<>{33});
    auto custom_beta = fixture::right::SharedView::create(fixture::right::SharedInput<>{44});
    assert(custom_alpha.view_unchecked().value() == 33);
    assert(custom_beta.view_unchecked().value() == 44);
    auto renamed = RenamedFieldView::create(annotation_fixture::RenamedField{7, {9}});
    assert(renamed.view_unchecked().exported_value() == 7);
    assert(renamed.view_unchecked().identifier().value() == 9);
    auto alpha = AlphaView::create(AlphaInput<>{{1, 2}});
    auto beta = BetaView::create(BetaInput<>{{3, 4}});
    assert(alpha.view_unchecked().values().get(1) == 2);
    assert(beta.view_unchecked().values().get(0) == 3);
    auto scalar = ScalarView::create(1.5f);
    assert(scalar.view_unchecked().value() == 1.5f);
    static_assert(AlignedView::alignment == 16);
    const std::array<std::byte, 4> bytes{std::byte{7}};
    assert(ReadOnlyViewReadOnly::from_bytes(bytes).value().value() == 7);
    auto read_only = AlphaViewReadOnly::from_bytes(alpha.bytes()).value();
    assert(read_only.values().get(1) == 2);
    auto vectors = ProjectionsView::create(ProjectionsInput<>{{VectorInput<>{1, 2, 3}}});
    assert(vectors.view_unchecked().vectors().size() == 1);
    assert(vectors.view_unchecked().vectors().get(0).z() == 3);
    vectors.view_unchecked().vectors().get(0).set_y(9.0f);
    assert(vectors.view_unchecked().vectors().get(0).y() == 9);
    ContractsInput<> contract_input{
        ContractChildInput<>{1}, ContractChildInput<>{2}, ContractChildInput<>{3}, {4,5,6},
        std::make_unique<std::vector<ContractChildInput<>>>(std::initializer_list<ContractChildInput<>>{{7}}),
        ConstantChildInput<>{8}, IdentifierInput<>{9}, {IdentifierInput<>{10}, IdentifierInput<>{11}}, MaybeValueInput<>{std::nullopt}, LabelInput<>{"hello"}
    };
    auto contract = ContractsView::create(contract_input);
    auto cv = contract.view_unchecked();
    static_assert(IdentifierView::byte_size == 16);
    assert(cv.identifier().field_0() == 9 && cv.identifiers().get(1).field_0() == 11);
    cv.identifier().set_field_0(19u);
    cv.editable().set_value(30u);
    cv.write_only().set_value(31u);
    cv.read_only().set_value(32u);
    cv.fixed().set_value(33u);
    cv.read_only_nested().value().get(0).set_value(34u);
    assert(cv.write_only().value() == 31 && cv.read_only().value() == 32 && cv.fixed().value() == 33);
    static_assert(!CanSetValue<decltype(ContractsViewReadOnly::from_bytes(contract.bytes()).value().editable())>);
    static_assert(CanSetValue<decltype(cv.read_only())>);
    static_assert(CanSetValue<decltype(cv.fixed())>);
    static_assert(CanSetValue<decltype(cv.read_only_nested().value().get(0))>);
    static_assert(CanReadValue<decltype(cv.write_only())>);
    assert(cv.identifier().field_0() == 19 && cv.editable().value() == 30);
    FixedArraysInput<> input{{1, 2, 3}, {4, 5, 6, 7}, {{{8, 9, 10}, {11, 12, 13}}},
        std::array<float, 3>{14, 15, 16}, std::nullopt, {17, 18, 19}};
    auto arrays = FixedArraysView::create(input);
    auto three = arrays.view_unchecked().three();
    static_assert(decltype(three)::extent == 3);
    static_assert(decltype(arrays.view_unchecked().four())::extent == 4);
    static_assert(decltype(arrays.view_unchecked().matrix())::extent == 2);
    static_assert(decltype(arrays.view_unchecked().matrix().get(0))::extent == 3);
    assert(three.get(2) == 3 && arrays.view_unchecked().matrix().get(1).get(2) == 13);
    assert(arrays.view_unchecked().present().value().get(2) == 16 && !arrays.view_unchecked().absent());
    three.set(1, 20.0f);
    assert(arrays.view_unchecked().three().get(1) == 20);

    // Tuples and explicit shared owners preserve independent wire values.
    namespace codecs = intrlay_generated::codecs;
    const auto shared = std::make_shared<std::uint32_t>(42);
    const codec_fixture::TupleOwners owners{
        {11, "tuple"}, {{53}, std::tuple{std::string("nested"), true}}, {}, shared, shared,
        std::make_shared<std::vector<std::uint32_t>>(std::initializer_list<std::uint32_t>{3, 5, 8})};
    auto owned = codecs::TupleOwnersView::create(owners);
    auto owned_view = codecs::TupleOwnersView::from_bytes(owned.bytes());
    assert(owned_view && owned_view->pair().field_1() == "tuple");
    assert(owned_view->nested().field_0().field_0() == 53);
    assert(owned_view->values().size() == 3);
    assert(shared.use_count() == 3);
    owned_view->set_first(71u);
    assert(owned_view->first() == 71 && owned_view->second() == 42 && *shared == 42);
    auto tuple_root = codecs::TupleRootView::create(std::tuple{std::uint32_t(17), std::string_view("root")});
    assert(tuple_root.view_unchecked().field_0() == 17);
    static_assert(!intrlay::detail::native_layout<codecs::TupleRootView::Metadata, std::tuple<std::uint32_t, std::string>>);

    // Tuple sources require exact arity at every depth, including zero fields.
    using BorrowedPair = std::tuple<std::uint32_t, std::string_view>;
    using ShortPair = std::tuple<std::uint32_t>;
    using LongPair = std::tuple<std::uint32_t, std::string_view, bool>;
    static_assert(CanCreateFrom<codecs::TupleRootView, BorrowedPair>);
    static_assert(!CanCreateFrom<codecs::TupleRootView, ShortPair>);
    static_assert(!CanCreateFrom<codecs::TupleRootView, LongPair>);
    using EmptyTupleView = decltype(owned_view->empty());
    static_assert(CanCreateFrom<EmptyTupleView, std::tuple<>>);
    static_assert(!CanCreateFrom<EmptyTupleView, ShortPair>);
    using NestedTupleView = decltype(owned_view->nested());
    using NestedSource = std::tuple<ShortPair, std::optional<std::tuple<std::string_view, bool>>>;
    using ExtraInnerField = std::tuple<std::tuple<std::uint32_t, bool>,
        std::optional<std::tuple<std::string_view, bool>>>;
    using ExtraOptionalField = std::tuple<ShortPair,
        std::optional<std::tuple<std::string_view, bool, bool>>>;
    static_assert(CanCreateFrom<NestedTupleView, NestedSource>);
    static_assert(!CanCreateFrom<NestedTupleView, ExtraInnerField>);
    static_assert(!CanCreateFrom<NestedTupleView, ExtraOptionalField>);
    using FixedTupleView = decltype(owned_view->nested().field_0());
    static_assert(CanReplaceFrom<FixedTupleView, ShortPair>);
    static_assert(!CanReplaceFrom<FixedTupleView, std::tuple<std::uint32_t, bool>>);
    static_assert(!intrlay::SetSourceFor<std::tuple<std::uint32_t, bool>, FixedTupleView::Metadata>);
    owned_view->nested().field_0().replace(ShortPair{73});
    assert(owned_view->nested().field_0().field_0() == 73);

    // Synthetic variant fields forward a complete payload, not a one-field tuple.
    using TuplePayload = intrlay::Variant<0, intrlay::Field<"value", 4,
        codecs::TupleRootView::Metadata, intrlay::AlternativeValue{}>>;
    static_assert(intrlay::EncodeSourceFor<BorrowedPair, TuplePayload>);
    static_assert(!intrlay::EncodeSourceFor<LongPair, TuplePayload>);

    // Named inputs may still carry unrelated fields beyond the schema.
    struct NamedSource { std::uint32_t value; std::string_view label; bool extra; };
    auto named = codecs::ValueRecordView::create(NamedSource{19, "structural", true});
    assert(named.view_unchecked().value() == 19 && named.view_unchecked().label() == "structural");

    // Native C arrays and a variant payload with its own member named value.
    auto native = codecs::CodecCasesView::create(
        codec_fixture::CodecCases{{1, 2, 3}, codec_fixture::ValueRecord{7, "payload"}});
    auto generated = codecs::CodecCasesView::create(
        codecs::CodecCasesInput<>{{1, 2, 3}, {codecs::ValueRecordInput<>{7, "payload"}}});
    assert(std::ranges::equal(native.bytes(), generated.bytes()));
    assert(native.view_unchecked().raw().get(2) == 3);
    auto payload = native.view_unchecked().choice().as_value_record().value();
    assert(payload.value() == 7 && payload.label() == "payload");

    // Duplicate payload types are selected by index, never by type.
    auto end = codecs::EndpointsView::create(codec_fixture::Endpoints{
        std::in_place_index<1>, codec_fixture::ValueRecord{9, "end"}});
    codecs::EndpointsInput<> generated_end;
    generated_end.value.emplace<1>(codecs::ValueRecordInput<>{9, "end"});
    auto end_copy = codecs::EndpointsView::create(generated_end);
    assert(std::ranges::equal(end.bytes(), end_copy.bytes()));
    assert(end.view_unchecked().tag() == 1 && !end.view_unchecked().as_start());
    assert(end.view_unchecked().as_end()->value() == 9 && end.view_unchecked().as_end()->label() == "end");

    // A custom traits string retains embedded NUL characters and matches std::string.
    const codec_fixture::Text text{{"a\0b", 3}};
    auto encoded_text = codecs::TextView::create(text);
    auto ordinary_text = codecs::TextView::create(codecs::TextInput<>{std::string("a\0b", 3)});
    assert(std::ranges::equal(encoded_text.bytes(), ordinary_text.bytes()));
    assert(encoded_text.view_unchecked().value() == std::string_view("a\0b", 3));
    assert(codecs::TextView::create(codec_fixture::Text{}).view_unchecked().value().empty());
}
