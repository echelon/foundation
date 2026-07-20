//! The one macro behind every enum in this crate: declares the enum with EXPLICIT per-variant
//! wire strings and generates `as_str`/`ALL`/`FromStr`/`Display`, serde (as the wire string),
//! and sqlx TEXT encode/decode (manual impls — deliberately not the sqlx derive, which targets
//! named Postgres enum types rather than plain TEXT columns).

macro_rules! text_enum {
  (
    $(#[$doc:meta])*
    $vis:vis enum $name:ident {
      $($variant:ident => $wire:literal),+ $(,)?
    }
  ) => {
    $(#[$doc])*
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    $vis enum $name {
      $($variant),+
    }

    impl $name {
      /// Every variant, for iteration/tests.
      pub const ALL: &'static [$name] = &[$($name::$variant),+];

      /// The wire string (database + API representation).
      pub const fn as_str(&self) -> &'static str {
        match self {
          $($name::$variant => $wire),+
        }
      }
    }

    impl ::std::str::FromStr for $name {
      type Err = $crate::ParseEnumError;
      fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
          $($wire => Ok($name::$variant),)+
          other => Err($crate::ParseEnumError { enum_name: stringify!($name), got: other.to_string() }),
        }
      }
    }

    impl ::std::fmt::Display for $name {
      fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        f.write_str(self.as_str())
      }
    }

    impl ::serde::Serialize for $name {
      fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
      }
    }

    impl<'de> ::serde::Deserialize<'de> for $name {
      fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <::std::borrow::Cow<'de, str> as ::serde::Deserialize>::deserialize(deserializer)?;
        s.parse::<$name>().map_err(::serde::de::Error::custom)
      }
    }

    // sqlx: TEXT-backed. Manual impls so `AS "col: enums::X"` overrides and binds both work
    // against ordinary `text` columns.
    impl ::sqlx::Type<::sqlx::Postgres> for $name {
      fn type_info() -> ::sqlx::postgres::PgTypeInfo {
        <String as ::sqlx::Type<::sqlx::Postgres>>::type_info()
      }
      fn compatible(ty: &::sqlx::postgres::PgTypeInfo) -> bool {
        <String as ::sqlx::Type<::sqlx::Postgres>>::compatible(ty)
      }
    }

    impl<'r> ::sqlx::Decode<'r, ::sqlx::Postgres> for $name {
      fn decode(value: ::sqlx::postgres::PgValueRef<'r>) -> Result<Self, ::sqlx::error::BoxDynError> {
        let s = <&str as ::sqlx::Decode<'r, ::sqlx::Postgres>>::decode(value)?;
        s.parse::<$name>().map_err(Into::into)
      }
    }

    impl<'q> ::sqlx::Encode<'q, ::sqlx::Postgres> for $name {
      fn encode_by_ref(
        &self,
        buf: &mut ::sqlx::postgres::PgArgumentBuffer,
      ) -> Result<::sqlx::encode::IsNull, ::sqlx::error::BoxDynError> {
        <&str as ::sqlx::Encode<'q, ::sqlx::Postgres>>::encode_by_ref(&self.as_str(), buf)
      }
    }
  };
}

pub(crate) use text_enum;
