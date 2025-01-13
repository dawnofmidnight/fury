kinds! {
    tokens {
        // token     ([macro     ] display             )
        Whitespace   ([whitespace] "whitespace"        )
        Comment      ([comment   ] "a comment"         )

        OpenParen    (['('       ] "`(`"               )
        CloseParen   ([')'       ] "`)`"               )
        OpenBracket  (['['       ] "`[`"               )
        CloseBracket ([']'       ] "`]`"               )
        OpenBrace    (['{'       ] "`{`"               )
        CloseBrace   (['}'       ] "`}`"               )

        Dot          ([.         ] "`.`"               )
        Comma        ([,         ] "`,`"               )
        Colon        ([:         ] "`:`"               )
        Semi         ([;         ] "`;`"               )
        Bang         ([!         ] "`!`"               )
        Equal        ([=         ] "`=`"               )
        Arrow        ([->        ] "`->`"              )

        Plus         ([+         ] "`+`"               )
        Minus        ([-         ] "`-`"               )
        Star         ([*         ] "`*`"               )
        Slash        ([/         ] "`/`"               )

        BoolLiteral  ([bool      ] "a boolean literal" )
        IntLiteral   ([int       ] "an integer literal")

        Ident        ([ident     ] "an identifier"     )
        FnKw         ([fn        ] "`fn`"              )

        Unknown      ([unknown   ] "an unknown token"  )
        Eof          ([eof       ] "the end of input"  )
    }

    nodes {
        Root
        Error

        Fn
        ParamList
        Param
        TypeExpr
        ExprLiteral
        ExprName
        ExprGroup
    }
}

impl Kind {
    #[cfg(test)]
    pub(super) fn is_token(self) -> bool {
        (self as u8) < (Self::_LastToken as u8)
    }

    pub(super) fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::Comment)
    }
}

macro_rules! kinds {
    (
        tokens { $($name:ident ($macro:tt $display:literal))* }
        nodes { $($node:ident)* }
    ) => {
        #[derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::fmt::Debug,
            ::core::cmp::PartialEq,
            ::core::cmp::Eq,
            ::core::hash::Hash,
        )]
        #[repr(u8)]
        pub(crate) enum Kind {
            $($name,)*
            _LastToken,
            $($node,)*
        }

        impl ::core::fmt::Display for Kind {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    $(Self::$name => f.write_str($display),)*
                    Self::_LastToken $(| Self::$node)* => panic!("tried to display a node Kind `{self:?}`")
                }
            }
        }

        macro_rules! t {
            $($macro => { $crate::syntax::kind::Kind::$name };)*
        }
        pub(super) use t;
    };
}

use kinds;
