/// Type's metatype
///
/// `GetType` provides a metatype that's a type of a type,
/// it also enables a developer to pass a type as a value to specify a generic type of a parameter
/// 
/// # Examples
/// ```
/// use spawn_groups::GetType;
/// use std::marker::PhantomData;
/// 
/// fn closure_taker<FUNC, T, U>(with_value: T, returning_type: PhantomData<U>, closure: FUNC) -> U
/// where FUNC: Fn(T) -> U {
///     closure(with_value)
/// }
///     
/// let string_result = closure_taker(32, String::TYPE, |val| format!("{}", val) );
/// 
/// assert_eq!(string_result, String::from("32"));
/// ```
/// 

use std::marker::PhantomData;

/// `GetType` trait implements asssociated constant for every type and this associated constant provides a metatype value that's a type's type value
/// of any type that is `?Sized`
pub trait GetType {
    /// Acts as a metatype value
    const TYPE: PhantomData<Self> = PhantomData;
}

impl<T: ?Sized> GetType for T {}
