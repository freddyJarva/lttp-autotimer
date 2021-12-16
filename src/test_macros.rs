/// used to assert values on object attributes, and prints an informative message on assertion failures
///
/// ## Example
/// ```
/// struct Person {
///     first_name: str,
///     last_name: str
/// }
/// let andy = Person {first_name: "andy", last_name: "andysson"};
/// assert_attrs!(andy: first_name == "andy", last_name != "bulbasaur")
/// ```
#[macro_export]
macro_rules! assert_attrs {
    ($object:ident: $($attr:ident $op:tt $value:expr,)*) => {
        $(
            assert!($object.$attr == $value, "expected {:?} == {:?}, but was {:?}", stringify!($object.$attr), $value, $object.$attr);
        )*
    };
}
