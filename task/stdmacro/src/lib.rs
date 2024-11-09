#![forbid(unsafe_code)]

#[macro_export]
macro_rules! deque {
    () => {
        ::std::collections::VecDeque::new()
    };
    ($elem:expr; $n:expr) => {
        ::std::iter::repeat($elem)
            .take($n)
            .collect::<::std::collections::VecDeque<_>>()
    };
    ($($x:expr),+ $(,)?) => {
        [$($x),*].into_iter().collect::<::std::collections::VecDeque<_>>()
    };
}

#[macro_export]
macro_rules! sorted_vec {
    () => {
        ::std::vec::Vec::new()
    };
    ($elem:expr; $n:expr) => {
        ::std::vec::from_elem($x, $n)
    };
    ($($x:expr),+ $(,)?) => ({
        let mut vec = <[_]>::into_vec(
            ::std::boxed::Box::new([$($x),+])
        );
        vec.sort();

        vec
    });
}

#[macro_export]
macro_rules! map {
    ($($key:expr=>$value:expr),* $(,)?) => {
        ::std::collections::HashMap::from([$(($key,$value)),*])
    };
}
