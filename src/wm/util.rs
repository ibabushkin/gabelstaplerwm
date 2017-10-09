/*
 * Copyright Inokentiy Babushkin and contributors (c) 2016-2017
 *
 * All rights reserved.

 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and the following disclaimer.
 *
 *     * Redistributions in binary form must reproduce the above
 *       copyright notice, this list of conditions and the following
 *       disclaimer in the documentation and/or other materials provided
 *       with the distribution.
 *
 *     * Neither the name of Inokentiy Babushkin nor the names of other
 *       contributors may be used to endorse or promote products derived
 *       from this software without specific prior written permission.

 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
 * A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
 * OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
 * SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
 * LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
 * DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
 * THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 * (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

#[macro_export]
macro_rules! declare_hierarchy {
    ($enum_ident:ident; $macro_ident:ident $(, $name:ident)*) => {
        pub enum $enum_ident {
            $($name($name)),*
        }

        #[macro_export]
        macro_rules! $macro_ident {
            ($layout:expr, $bind:pat => $body:expr) => {
                match $layout {
                    $($enum_ident::$name($bind) => $body),*
                }
            }
        }
    }
}

#[macro_export]
macro_rules! declare_hierarchy_with_parser {
    ($enum_ident:ident; $macro_ident:ident $(, ($name: ident; $cmd:expr))*) => {
        declare_hierarchy!($enum_ident; $macro_ident $(, $name)*);

        impl $enum_ident {
            pub fn parse_from_words(words: &[&str]) -> Option<Self> {
                $(
                    if words[0] == $cmd {
                        return $name::parse_from_words(&words[1..]).map($enum_ident::$name);
                    }
                )*

                None
            }
        }
    }
}
