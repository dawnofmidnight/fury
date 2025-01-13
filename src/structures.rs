macro_rules! arena {
    ($ty:ty, $arena_vis:vis $arena:ident, $key_vis:vis $key:ident) => {
        /// An index-based arena structure containing
        #[doc = concat!("[`", stringify!($ty), "`]s,")]
        /// implemented via a `Vec`. Elements of the arena can be accessed
        /// using a
        #[doc = concat!("[`", stringify!($key), "`].")]
        #[derive(
            ::core::clone::Clone,
            ::core::fmt::Debug,
            ::core::default::Default,
            ::core::cmp::PartialEq,
            ::core::cmp::Eq
        )]
        $arena_vis struct $arena {
            data: ::std::vec::Vec<$ty>,
        }

        impl $arena {
            /// Create a new empty arena.
            #[allow(unused)]
            $arena_vis const fn new() -> Self {
                Self { data: ::std::vec::Vec::new() }
            }

            /// Create a new empty arena with a specified capacity.
            #[allow(unused)]
            $arena_vis fn with_capacity(capacity: ::core::primitive::usize) -> Self {
                Self { data: ::std::vec::Vec::with_capacity(capacity) }
            }

            /// Returns the length of the arena.
            #[allow(unused)]
            $arena_vis fn len(&self) -> ::core::primitive::usize {
                self.data.len()
            }

            /// Returns whether the arena is currently empty.
            #[allow(unused)]
            $arena_vis fn is_empty(&self) -> ::core::primitive::bool {
                self.data.is_empty()
            }

            /// Allocates a new value in the arena, returning its index.
            #[allow(unused)]
            $arena_vis fn alloc(&mut self, value: $ty) -> $key {
                self.data.push(value);
                $key(::core::num::NonZeroU32::new(
                    ::core::primitive::u32::try_from(self.data.len()).unwrap()
                ).unwrap())
            }
        }

        impl ::core::ops::Index<$key> for $arena {
            type Output = $ty;

            fn index(&self, key: $key) -> &$ty {
                &self.data[::core::primitive::usize::try_from((key.0.get() - 1)).unwrap()]
            }
        }

        impl ::core::ops::IndexMut<$key> for $arena {
            fn index_mut(&mut self, key: $key) -> &mut $ty {
                &mut self.data[::core::primitive::usize::try_from((key.0.get() - 1)).unwrap()]
            }
        }

        /// A key to an element of a
        #[doc = concat!("[`", stringify!($arena), "`].")]
        #[derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::fmt::Debug,
            ::core::cmp::PartialEq,
            ::core::cmp::Eq,
            ::core::hash::Hash
        )]
        $key_vis struct $key(::core::num::NonZeroU32);
    }
}
pub(crate) use arena;

#[cfg(test)]
mod test_arena {
    arena!(&'static str, StrArena, StrKey);
    #[test]
    fn empty() {
        let arena = StrArena::new();
        assert!(arena.is_empty());
    }

    #[test]
    fn len() {
        let mut arena = StrArena::new();
        assert_eq!(arena.len(), 0);
        arena.alloc("a");
        assert_eq!(arena.len(), 1);
        arena.alloc("b");
        assert_eq!(arena.len(), 2);
        arena.alloc("c");
        assert_eq!(arena.len(), 3);
    }

    #[test]
    fn indexing() {
        let mut arena = StrArena::new();
        let a = arena.alloc("a");
        let b = arena.alloc("b");
        assert_eq!(arena[a], "a");
        assert_eq!(arena[b], "b");
        arena[a] = "c";
        assert_eq!(arena[a], "c");
        assert_eq!(arena[b], "b");
    }
}
