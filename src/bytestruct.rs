/// A unique declarative macro for generating bitfields backed by fixed-size byte arrays.
///
/// Unlike standard bitfield libraries that restrict storage to primitives (u8-u128),
/// `bytestruct` allows array-backed storage (`[u8; 1-16]`) while maintaining
/// register-wide optimization through "Acting Primitives".
///
/// This macro generates a struct wrapping `[u8; N]`. It uses an internal "acting primitive"
/// (u32, u64, or u128) based on $N$ to perform efficient bitwise operations.
///
/// # Examples
///
/// ### Cross-Boundary Packing in a Byte Array
/// ```rust
/// use bitcraft::bytestruct;
///
/// bytestruct! {
///     // Packs fields into a 5-byte (40-bit) storage array.
///     pub struct NetworkPacket([u8; 5]) {
///         pub version: u8 = 4,         // Bits 0-3
///         pub is_encrypted: bool = 1,  // Bit 4
///         pub payload_id: u32 = 24,    // Bits 5-28 (crosses 3 byte boundaries!)
///         pub checksum: u16 = 11,      // Bits 29-39 (crosses 2 byte boundaries!)
///     }
/// }
///
/// let mut packet = NetworkPacket::default()
///     .with_version(4)
///     .with_is_encrypted(true)
///     .with_payload_id(0xABCDEF);
///
/// assert_eq!(packet.version(), 4);
/// assert!(packet.is_encrypted());
/// assert_eq!(packet.payload_id(), 0xABCDEF);
///
/// // The raw array perfectly packs these bits densely
/// let raw: [u8; 5] = packet.to_array();
/// ```
///
/// ### Advanced Typed Storage (`[u16; N]`, `[u32; N]`) & Signed Fields
/// `bytestruct!` natively supports generic integer array types, which enables creating large dense
/// multi-element bitfields with highly efficient memory bounds.
/// ```rust
/// use bitcraft::bytestruct;
///
/// bytestruct! {
///     // A 128-bit wrapper stored as four `u32` blocks.
///     pub struct HugeStorage([u32; 4]) {
///         pub header: u16 = 16,
///         pub sensor_val: i32 = 20, // A 20-bit signed field (-524,288 to 524,287)
///         pub hash: u64 = 64,       // 64-bit contiguous field crossing u32 boundaries
///         // ... other fields filling the 128 bits ...
///     }
/// }
///
/// let mut mem = HugeStorage::default();
/// mem.set_sensor_val(-400_000);
/// assert_eq!(mem.sensor_val(), -400_000);
/// ```
///
/// # Implementation Details
///
/// `bytestruct!` uses an advanced "Token-Tree (TT) Muncher" pattern coupled with dynamic
/// type resolution via the hidden `BitenumType` trait. It completely unrolls array reads
/// and writes into single-primitive logic.
///
/// For instance, if you define a 5-byte array (`[u8; 5]`), `bytestruct!` computes the total
/// number of bits (40 bits). Through `crate::Bits<40>`, the Rust type system dynamically resolves
/// the "Acting Primitive" to `u64`. All internal field getters and setters will seamlessly
/// load the required array slice into a `u64` register, perform the bitwise logic, and write
/// the bytes back without branching.
#[macro_export]
macro_rules! bytestruct {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident ([$unit:tt; $N:tt]) {
            $(
                $field_vis:vis $field_name:ident: $field_type:tt = $bits:tt
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone, PartialEq, Eq, Default)]
        #[derive($crate::bytemuck::Pod, $crate::bytemuck::Zeroable)]
        #[repr(C)]
        $vis struct $name(pub [$unit; $N]);

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("raw", &self.0)
                    $(
                        .field(stringify!($field_name), &self.$field_name())
                    )*
                    .finish()
            }
        }

        const _: () = {
            #[allow(dead_code)]
            const UNIT_BITS: usize = <$unit as $crate::BitLength>::BITS;
            #[allow(dead_code)]
            const TOTAL_BITS: usize = $crate::bits_mul!($unit, $N);
            // Simplified check for typed storage to avoid macro repetition issues
            $crate::bitstruct!(@check_fields $($field_type)*);
        };

        impl $name {
            #[allow(dead_code)]
            pub const BITS: usize = $crate::bits_mul!($unit, $N);
            #[allow(dead_code)]
            pub const UNIT_BITS: usize = <$unit as $crate::BitLength>::BITS;
            #[allow(dead_code)]
            #[inline(always)] pub const fn to_array(self) -> [$unit; $N] { self.0 }
            #[allow(dead_code)]
            #[inline(always)] pub const fn from_array(arr: [$unit; $N]) -> Self { Self(arr) }

            $crate::bytestruct!(@impl_typed_conversions $name, $unit, $N);
            $crate::bytestruct!(@impl_typed_fields $name, $unit, <$crate::Bits<{ Self::BITS }> as $crate::BitenumType>::Prim, 0, $($field_vis $field_name $field_type $bits)*);
        }
    };

    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident ([$unit:tt; $N:tt]) {
            $field_vis:vis $field_name:ident : ($field_type:ty) = @unroll($count:tt)
        }
    ) => {
        $crate::paste::paste! {
            $(#[$meta])*
            #[derive(Copy, Clone, PartialEq, Eq, Default)]
            #[derive($crate::bytemuck::Pod, $crate::bytemuck::Zeroable)]
            #[repr(C)]
            $vis struct $name(pub [$unit; $N]);

            impl core::fmt::Debug for $name {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct(stringify!($name))
                        .field("raw", &self.0)
                        .field(stringify!($field_name), &self.$field_name())
                        .finish()
                }
            }

            impl $name {
                #[allow(dead_code)]
                pub const BITS: usize = $crate::bits_mul!($unit, $N);
                #[allow(dead_code)]
                pub const UNIT_BITS: usize = <$unit as $crate::BitLength>::BITS;
                #[allow(dead_code)]
                #[inline(always)] pub const fn to_array(self) -> [$unit; $N] { self.0 }
                #[allow(dead_code)]
                #[inline(always)] pub const fn from_array(arr: [$unit; $N]) -> Self { Self(arr) }

                $crate::bytestruct!(@impl_typed_conversions $name, $unit, $N);
                $crate::bytestruct!(@impl_typed_fields $name, $unit, <$crate::Bits<{ Self::BITS }> as $crate::BitenumType>::Prim, 0, $field_vis $field_name ($field_type) = @unroll($count));
            }
        }
    };

    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident ([$unit:tt; $N:tt]) {
            $field_vis:vis $field_name:ident : ($field_type:ty) = @unroll_signed($count:tt)
        }
    ) => {
        $crate::paste::paste! {
            $(#[$meta])*
            #[derive(Copy, Clone, PartialEq, Eq, Default)]
            #[derive($crate::bytemuck::Pod, $crate::bytemuck::Zeroable)]
            #[repr(C)]
            $vis struct $name(pub [$unit; $N]);

            impl core::fmt::Debug for $name {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct(stringify!($name))
                        .field("raw", &self.0)
                        .field(stringify!($field_name), &self.$field_name())
                        .finish()
                }
            }

            impl $name {
                #[allow(dead_code)]
                pub const BITS: usize = $crate::bits_mul!($unit, $N);
                #[allow(dead_code)]
                pub const UNIT_BITS: usize = <$unit as $crate::BitLength>::BITS;
                #[allow(dead_code)]
                #[inline(always)] pub const fn to_array(self) -> [$unit; $N] { self.0 }
                #[allow(dead_code)]
                #[inline(always)] pub const fn from_array(arr: [$unit; $N]) -> Self { Self(arr) }

                $crate::bytestruct!(@impl_typed_conversions $name, $unit, $N);
                $crate::bytestruct!(@impl_typed_fields $name, $unit, <$crate::Bits<{ Self::BITS }> as $crate::BitenumType>::Prim, 0, $field_vis $field_name ($field_type) = @unroll_signed($count));
            }
        }
    };

    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident ($N:tt) {
            $(
                $field_vis:vis $field_name:ident: $field_type:tt = $bits:tt
            ),* $(,)?
        }
    ) => {
        $crate::bytestruct! {
            $(#[$meta])*
            $vis struct $name ([u8; $N]) {
                $($field_vis $field_name: $field_type = $bits),*
            }
        }
    };

    // --- INTERNAL CONVERSIONS ---
    (@impl_conversions $name:ident, 1) => {
        #[inline(always)] pub const fn to_u8(self) -> u8 { self.0[0] }
        #[inline(always)] pub const fn from_u8(val: u8) -> Self { Self([val]) }
        #[inline(always)] pub const fn to_bits(self) -> u8 { self.to_u8() }
        #[inline(always)] pub const fn from_bits(val: u8) -> Self { Self::from_u8(val) }
    };
    (@impl_conversions $name:ident, 2) => {
        #[inline(always)] pub const fn to_u16(self) -> u16 { u16::from_le_bytes(self.0) }
        #[inline(always)] pub const fn from_u16(val: u16) -> Self { Self(val.to_le_bytes()) }
        #[inline(always)] pub const fn to_bits(self) -> u16 { self.to_u16() }
        #[inline(always)] pub const fn from_bits(val: u16) -> Self { Self::from_u16(val) }
    };
    (@impl_conversions $name:ident, 3) => {
        $crate::bytestruct!(@impl_wide_conv $name, 3, u32, to_u32, from_u32, 0 1 2);
    };
    (@impl_conversions $name:ident, 4) => {
        #[inline(always)] pub const fn to_u32(self) -> u32 { u32::from_le_bytes(self.0) }
        #[inline(always)] pub const fn from_u32(val: u32) -> Self { Self(val.to_le_bytes()) }
        #[inline(always)] pub const fn to_bits(self) -> u32 { self.to_u32() }
        #[inline(always)] pub const fn from_bits(val: u32) -> Self { Self::from_u32(val) }
    };
    (@impl_conversions $name:ident, 5) => { $crate::bytestruct!(@impl_wide_conv $name, 5, u64, to_u64, from_u64, 0 1 2 3 4); };
    (@impl_conversions $name:ident, 6) => { $crate::bytestruct!(@impl_wide_conv $name, 6, u64, to_u64, from_u64, 0 1 2 3 4 5); };
    (@impl_conversions $name:ident, 7) => { $crate::bytestruct!(@impl_wide_conv $name, 7, u64, to_u64, from_u64, 0 1 2 3 4 5 6); };
    (@impl_conversions $name:ident, 8) => {
        #[inline(always)] pub const fn to_u64(self) -> u64 { u64::from_le_bytes(self.0) }
        #[inline(always)] pub const fn from_u64(val: u64) -> Self { Self(val.to_le_bytes()) }
        #[inline(always)] pub const fn to_bits(self) -> u64 { self.to_u64() }
        #[inline(always)] pub const fn from_bits(val: u64) -> Self { Self::from_u64(val) }
    };
    (@impl_conversions $name:ident, 16) => {
        #[inline(always)] pub const fn to_u128(self) -> u128 { u128::from_le_bytes(self.0) }
        #[inline(always)] pub const fn from_u128(val: u128) -> Self { Self(val.to_le_bytes()) }
        #[inline(always)] pub const fn to_bits(self) -> u128 { self.to_u128() }
        #[inline(always)] pub const fn from_bits(val: u128) -> Self { Self::from_u128(val) }
    };
    (@impl_conversions $name:ident, $N:expr) => {
        #[inline(always)]
        #[allow(dead_code)]
        pub const fn to_bits(self) -> <$crate::Bits<{ $crate::bits_mul!(u8, $N) }> as $crate::BitenumType>::Prim {
            let mut acc: <$crate::Bits<{ $crate::bits_mul!(u8, $N) }> as $crate::BitenumType>::Prim = 0;
            let mut i = 0;
            while i < $N {
                acc |= (self.0[i] as <$crate::Bits<{ $crate::bits_mul!(u8, $N) }> as $crate::BitenumType>::Prim) << $crate::bits_mul!(u8, i);
                i += 1;
            }
            acc
        }
        #[inline(always)]
        #[allow(dead_code)]
        pub const fn from_bits(val: <$crate::Bits<{ $crate::bits_mul!(u8, $N) }> as $crate::BitenumType>::Prim) -> Self {
            let mut arr = [0u8; $N];
            let mut i = 0;
            while i < $N {
                arr[i] = (val >> $crate::bits_mul!(u8, i)) as u8;
                i += 1;
            }
            Self(arr)
        }
    };


    (@impl_wide_conv $name:ident, $N:tt, $prim:ty, $to_name:ident, $from_name:ident, $($idx:literal)*) => {
        #[inline(always)] pub const fn $to_name(self) -> $prim { 0 $( | (self.0[$idx] as $prim) << ($idx << 3) )* }
        #[inline(always)] pub const fn $from_name(val: $prim) -> Self { Self([ $( (val >> ($idx << 3)) as u8 ),* ]) }
        #[inline(always)] pub const fn to_bits(self) -> $prim { self.$to_name() }
        #[inline(always)] pub const fn from_bits(val: $prim) -> Self { Self::$from_name(val) }
    };

    (@impl_fields $name:ident, $prim:ty, $shift:expr, ) => {};

    // Standard munchers
    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident bool $bits:tt $($rest:tt)*) => {
        $crate::paste::paste! {
            /// The bit-offset of the `$field_name` property within the underlying storage.
            pub const [<$field_name:upper _OFFSET>]: usize = $shift;
            /// The number of bits allocated for the `$field_name` property.
            pub const [<$field_name:upper _BITS>]: usize = $bits;
            #[doc(hidden)]
            const [<$field_name:upper _MASK>]: u128 = (!0u128) >> (128 - Self::[<$field_name:upper _BITS>]);
            $field_vis const fn $field_name(self) -> bool { $crate::bytestruct!(@read_localized_prim self.0, $shift, $bits) != 0 }
            $field_vis fn [<set_ $field_name>](&mut self, val: bool) {
                $crate::bytestruct!(@write_localized_prim self.0, $shift, $bits, val as u128);
            }
            $field_vis const fn [<with_ $field_name>](mut self, val: bool) -> Self {
                $crate::bytestruct!(@write_localized_prim self.0, $shift, $bits, val as u128);
                self
            }
        }
        $crate::bytestruct!(@impl_fields $name, $prim, $shift + $bits, $($rest)*);
    };

    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident u8 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_int $name, $prim, $shift, $field_vis $field_name u8 $bits $($rest)*); };
    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident u16 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_int $name, $prim, $shift, $field_vis $field_name u16 $bits $($rest)*); };
    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident u32 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_int $name, $prim, $shift, $field_vis $field_name u32 $bits $($rest)*); };
    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident u64 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_int $name, $prim, $shift, $field_vis $field_name u64 $bits $($rest)*); };
    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident u128 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_int $name, $prim, $shift, $field_vis $field_name u128 $bits $($rest)*); };
    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident i8 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_signed_int $name, $prim, $shift, $field_vis $field_name i8 $bits $($rest)*); };
    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident i16 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_signed_int $name, $prim, $shift, $field_vis $field_name i16 $bits $($rest)*); };
    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident i32 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_signed_int $name, $prim, $shift, $field_vis $field_name i32 $bits $($rest)*); };
    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident i64 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_signed_int $name, $prim, $shift, $field_vis $field_name i64 $bits $($rest)*); };
    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident i128 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_signed_int $name, $prim, $shift, $field_vis $field_name i128 $bits $($rest)*); };

    (@impl_int $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident $field_type:tt $bits:tt $($rest:tt)*) => {
        $crate::paste::paste! {
            /// The bit-offset of the `$field_name` property within the underlying storage.
            pub const [<$field_name:upper _OFFSET>]: usize = $shift;
            /// The number of bits allocated for the `$field_name` property.
            pub const [<$field_name:upper _BITS>]: usize = $bits;
            #[doc(hidden)]
            const [<$field_name:upper _MASK>]: u128 = (!0u128) >> (128 - Self::[<$field_name:upper _BITS>]);
            $field_vis const fn $field_name(self) -> $field_type { $crate::bytestruct!(@read_localized_prim self.0, $shift, $bits) as $field_type }
            $field_vis fn [<set_ $field_name>](&mut self, val: $field_type) {
                debug_assert!((val as u128) <= Self::[<$field_name:upper _MASK>], "Value {} overflows allocated {} bits", val, $bits);
                $crate::bytestruct!(@write_localized_prim self.0, $shift, $bits, val as u128);
            }
            $field_vis const fn [<with_ $field_name>](mut self, val: $field_type) -> Self {
                debug_assert!((val as u128) <= Self::[<$field_name:upper _MASK>], "Value overflows allocated bits");
                $crate::bytestruct!(@write_localized_prim self.0, $shift, $bits, val as u128);
                self
            }
            $field_vis fn [<try_set_ $field_name>](&mut self, val: $field_type) -> Result<(), $crate::BitstructError> {
                if (val as u128) > Self::[<$field_name:upper _MASK>] { return Err($crate::BitstructError::Overflow { value: val as u128, allocated_bits: $bits }); }
                self.[<set_ $field_name>](val); Ok(())
            }
            $field_vis const fn [<try_with_ $field_name>](self, val: $field_type) -> Result<Self, $crate::BitstructError> {
                if (val as u128) > Self::[<$field_name:upper _MASK>] { return Err($crate::BitstructError::Overflow { value: val as u128, allocated_bits: $bits }); }
                Ok(self.[<with_ $field_name>](val))
            }
        }
        $crate::bytestruct!(@impl_fields $name, $prim, $shift + $bits, $($rest)*);
    };

    // Signed integer implementation for standard (localized) `bytestruct!` mappings.
    // Utilizes arithmetic right-shifts on target primitives to synthesize seamless two's complement behavior
    // across varying alignment constraints without complex branching logic.
    (@impl_signed_int $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident $field_type:tt $bits:tt $($rest:tt)*) => {
        $crate::paste::paste! {
            /// The bit-offset of the `$field_name` property within the underlying storage.
            pub const [<$field_name:upper _OFFSET>]: usize = $shift;
            /// The number of bits allocated for the `$field_name` property.
            pub const [<$field_name:upper _BITS>]: usize = $bits;
            #[doc(hidden)]
            const [<$field_name:upper _MASK>]: u128 = (!0u128) >> (128 - Self::[<$field_name:upper _BITS>]);
            #[doc(hidden)]
            pub const [<$field_name:upper _MIN>]: $field_type = (!0 as $field_type) << (Self::[<$field_name:upper _BITS>] - 1);
            #[doc(hidden)]
            pub const [<$field_name:upper _MAX>]: $field_type = !Self::[<$field_name:upper _MIN>];
            #[doc(hidden)]
            const [<$field_name:upper _SHIFT_UP>]: usize = <$field_type as $crate::BitLength>::BITS - Self::[<$field_name:upper _BITS>];

            $field_vis const fn $field_name(self) -> $field_type {
                let raw = $crate::bytestruct!(@read_localized_prim self.0, $shift, $bits) as $field_type;
                (raw << Self::[<$field_name:upper _SHIFT_UP>]) >> Self::[<$field_name:upper _SHIFT_UP>]
            }
            $field_vis fn [<set_ $field_name>](&mut self, val: $field_type) {
                debug_assert!(val >= Self::[<$field_name:upper _MIN>] && val <= Self::[<$field_name:upper _MAX>], "Value {} out of bounds for {} bits", val, $bits);
                $crate::bytestruct!(@write_localized_prim self.0, $shift, $bits, val as u128);
            }
            $field_vis const fn [<with_ $field_name>](mut self, val: $field_type) -> Self {
                debug_assert!(val >= Self::[<$field_name:upper _MIN>] && val <= Self::[<$field_name:upper _MAX>], "Value overflows allocated bits");
                $crate::bytestruct!(@write_localized_prim self.0, $shift, $bits, val as u128);
                self
            }
            $field_vis fn [<try_set_ $field_name>](&mut self, val: $field_type) -> Result<(), $crate::BitstructError> {
                if val < Self::[<$field_name:upper _MIN>] || val > Self::[<$field_name:upper _MAX>] { return Err($crate::BitstructError::Overflow { value: val as i128 as u128, allocated_bits: $bits }); }
                self.[<set_ $field_name>](val); Ok(())
            }
            $field_vis const fn [<try_with_ $field_name>](self, val: $field_type) -> Result<Self, $crate::BitstructError> {
                if val < Self::[<$field_name:upper _MIN>] || val > Self::[<$field_name:upper _MAX>] { return Err($crate::BitstructError::Overflow { value: val as i128 as u128, allocated_bits: $bits }); }
                Ok(self.[<with_ $field_name>](val))
            }
        }
        $crate::bytestruct!(@impl_fields $name, $prim, $shift + $bits, $($rest)*);
    };

    // --- TYPED STORAGE HELPERS ---
    // These helpers provide an optimized bridge between an array-backed struct ([u8; N], [u16; N], etc.)
    // and a single primitive CPU register (u32, u64, or u128).
    (@impl_typed_conversions $name:ident, $unit:tt, $N:tt) => {
        /// Converts the packed structure into its raw bit representation as the narrowest possible
        /// primitive integer (u32, u64, or u128) that can hold all bits.
        ///
        /// This method utilizes the fully-unrolled bitwise engine for maximum register efficiency.
        #[allow(dead_code)]
        #[inline(always)]
        pub const fn to_bits(self) -> <$crate::Bits<{ Self::BITS }> as $crate::BitenumType>::Prim {
            $crate::bytestruct!(@unroll_typed_read self.0, $unit, <$crate::Bits<{ Self::BITS }> as $crate::BitenumType>::Prim, 0, $N, 0, Self::BITS) as _
        }

        /// Reconstructs the packed structure from a raw bit representation.
        ///
        /// This uses a specialized 'fresh write' path that initializes the underlying array
        /// in a single unrolled pass, avoiding redundant masking operations on empty storage.
        #[allow(dead_code)]
        #[inline(always)]
        pub const fn from_bits(val: <$crate::Bits<{ Self::BITS }> as $crate::BitenumType>::Prim) -> Self {
            let mut arr = [0 as $unit; $N];
            $crate::bytestruct!(@unroll_typed_write_fresh arr, $unit, <$crate::Bits<{ Self::BITS }> as $crate::BitenumType>::Prim, 0, $N, 0, Self::BITS, val);
            Self(arr)
        }

        $crate::bytestruct!(@route_conversions $name, $unit, $N);
    };

    // Routing Logic for Typed Conversions
    (@route_conversions $name:ident, $unit:tt, 1) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 1); };
    (@route_conversions $name:ident, $unit:tt, 2) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 2); };
    (@route_conversions $name:ident, $unit:tt, 3) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 3); };
    (@route_conversions $name:ident, $unit:tt, 4) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 4); };
    (@route_conversions $name:ident, $unit:tt, 5) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 5); };
    (@route_conversions $name:ident, $unit:tt, 6) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 6); };
    (@route_conversions $name:ident, $unit:tt, 7) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 7); };
    (@route_conversions $name:ident, $unit:tt, 8) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 8); };
    (@route_conversions $name:ident, $unit:tt, 9) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 9); };
    (@route_conversions $name:ident, $unit:tt, 10) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 10); };
    (@route_conversions $name:ident, $unit:tt, 11) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 11); };
    (@route_conversions $name:ident, $unit:tt, 12) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 12); };
    (@route_conversions $name:ident, $unit:tt, 13) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 13); };
    (@route_conversions $name:ident, $unit:tt, 14) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 14); };
    (@route_conversions $name:ident, $unit:tt, 15) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 15); };
    (@route_conversions $name:ident, $unit:tt, 16) => { $crate::bytestruct!(@impl_typed_uX_methods $name, $unit, 16); };
    (@route_conversions $name:ident, $unit:tt, $any:expr) => {};

    // Narrowest bit-mapping for common storage units
    (@impl_typed_uX_methods $name:ident, u8, 1) => { $crate::bytestruct!(@impl_u8_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 2) => { $crate::bytestruct!(@impl_u16_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 3) => { $crate::bytestruct!(@impl_u32_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 4) => { $crate::bytestruct!(@impl_u32_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 5) => { $crate::bytestruct!(@impl_u64_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 6) => { $crate::bytestruct!(@impl_u64_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 7) => { $crate::bytestruct!(@impl_u64_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 8) => { $crate::bytestruct!(@impl_u64_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 9) => { $crate::bytestruct!(@impl_u128_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 10) => { $crate::bytestruct!(@impl_u128_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 11) => { $crate::bytestruct!(@impl_u128_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 12) => { $crate::bytestruct!(@impl_u128_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 13) => { $crate::bytestruct!(@impl_u128_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 14) => { $crate::bytestruct!(@impl_u128_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 15) => { $crate::bytestruct!(@impl_u128_methods $name); };
    (@impl_typed_uX_methods $name:ident, u8, 16) => { $crate::bytestruct!(@impl_u128_methods $name); };

    (@impl_typed_uX_methods $name:ident, u16, 1) => { $crate::bytestruct!(@impl_u16_methods $name); };
    (@impl_typed_uX_methods $name:ident, u16, 2) => { $crate::bytestruct!(@impl_u32_methods $name); };
    (@impl_typed_uX_methods $name:ident, u16, 3) => { $crate::bytestruct!(@impl_u64_methods $name); };
    (@impl_typed_uX_methods $name:ident, u16, 4) => { $crate::bytestruct!(@impl_u64_methods $name); };
    (@impl_typed_uX_methods $name:ident, u16, 5) => { $crate::bytestruct!(@impl_u128_methods $name); };
    (@impl_typed_uX_methods $name:ident, u16, 6) => { $crate::bytestruct!(@impl_u128_methods $name); };
    (@impl_typed_uX_methods $name:ident, u16, 7) => { $crate::bytestruct!(@impl_u128_methods $name); };
    (@impl_typed_uX_methods $name:ident, u16, 8) => { $crate::bytestruct!(@impl_u128_methods $name); };

    (@impl_typed_uX_methods $name:ident, u32, 1) => { $crate::bytestruct!(@impl_u32_methods $name); };
    (@impl_typed_uX_methods $name:ident, u32, 2) => { $crate::bytestruct!(@impl_u64_methods $name); };
    (@impl_typed_uX_methods $name:ident, u32, 3) => { $crate::bytestruct!(@impl_u128_methods $name); };
    (@impl_typed_uX_methods $name:ident, u32, 4) => { $crate::bytestruct!(@impl_u128_methods $name); };

    (@impl_typed_uX_methods $name:ident, u64, 1) => { $crate::bytestruct!(@impl_u64_methods $name); };
    (@impl_typed_uX_methods $name:ident, u64, 2) => { $crate::bytestruct!(@impl_u128_methods $name); };

    (@impl_typed_uX_methods $name:ident, u128, 1) => { $crate::bytestruct!(@impl_u128_methods $name); };

    (@impl_u8_methods $name:ident) => {
        #[allow(dead_code)]
        #[inline(always)] pub const fn to_u8(self) -> u8 { self.to_bits() as u8 }
        #[allow(dead_code)]
        #[inline(always)] pub const fn from_u8(val: u8) -> Self { Self::from_bits(val as _) }
    };
    (@impl_u16_methods $name:ident) => {
        #[allow(dead_code)]
        #[inline(always)] pub const fn to_u16(self) -> u16 { self.to_bits() as u16 }
        #[allow(dead_code)]
        #[inline(always)] pub const fn from_u16(val: u16) -> Self { Self::from_bits(val as _) }
    };
    (@impl_u32_methods $name:ident) => {
        #[allow(dead_code)]
        #[inline(always)] pub const fn to_u32(self) -> u32 { self.to_bits() as u32 }
        #[allow(dead_code)]
        #[inline(always)] pub const fn from_u32(val: u32) -> Self { Self::from_bits(val as _) }
    };
    (@impl_u64_methods $name:ident) => {
        #[allow(dead_code)]
        #[inline(always)] pub const fn to_u64(self) -> u64 { self.to_bits() as u64 }
        #[allow(dead_code)]
        #[inline(always)] pub const fn from_u64(val: u64) -> Self { Self::from_bits(val as _) }
    };
    (@impl_u128_methods $name:ident) => {
        #[allow(dead_code)]
        #[inline(always)] pub const fn to_u128(self) -> u128 { self.to_bits() as u128 }
        #[allow(dead_code)]
        #[inline(always)] pub const fn from_u128(val: u128) -> Self { Self::from_bits(val as _) }
    };


    // --- UNROLLED FIELD IMPLEMENTATION ---
    // Specifically used for high-performance 'register-like' fields that occupy
    // the entirety of their defined storage (typical of types generated by byteval!).
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident ($field_type:ty) = @unroll($count:tt) $($rest:tt)*) => {
        $crate::paste::paste! {
            #[allow(dead_code)]
            /// The bit-offset of the `$field_name` property within the underlying storage.
            pub const [<$field_name:upper _OFFSET>]: usize = $shift;
            #[allow(dead_code)]
            /// The number of bits allocated for the `$field_name` property (unrolled).
            pub const [<$field_name:upper _BITS>]: usize = $crate::bits_mul!($unit, $count);
            #[allow(dead_code)]
            #[doc(hidden)]
            const [<$field_name:upper _MASK>]: $field_type = (!0 as $field_type) >> (<$field_type as $crate::BitLength>::BITS - Self::[<$field_name:upper _BITS>]);

            #[allow(dead_code)]
            #[inline(always)]
            pub const fn $field_name(self) -> $field_type {
                 $crate::bytestruct!(@unroll_typed_read self.0, $unit, $prim, 0, $count, 0, Self::[<$field_name:upper _BITS>]) as _
            }
            #[allow(dead_code)]
            #[inline(always)]
            pub fn [<set_ $field_name>](&mut self, val: $field_type) {
                 debug_assert!((val as u128) <= Self::[<$field_name:upper _MASK>] as u128, "Value {} overflows allocated {} bits", val, Self::[<$field_name:upper _BITS>]);
                 $crate::bytestruct!(@unroll_typed_write self.0, $unit, $prim, 0, $count, 0, Self::[<$field_name:upper _BITS>], val as $prim);
            }
            #[allow(dead_code)]
            #[inline(always)]
            pub const fn [<with_ $field_name>](mut self, val: $field_type) -> Self {
                 debug_assert!((val as u128) <= Self::[<$field_name:upper _MASK>] as u128, "Value overflows allocated bits");
                 $crate::bytestruct!(@unroll_typed_write self.0, $unit, $prim, 0, $count, 0, Self::[<$field_name:upper _BITS>], val as $prim);
                 self
            }
            #[allow(dead_code)]
            #[inline(always)]
            pub fn [<try_set_ $field_name>](&mut self, val: $field_type) -> Result<(), $crate::BitstructError> {
                 if (val as u128) > Self::[<$field_name:upper _MASK>] as u128 { return Err($crate::BitstructError::Overflow { value: val as u128, allocated_bits: Self::[<$field_name:upper _BITS>] }); }
                 self.[<set_ $field_name>](val); Ok(())
            }
            #[allow(dead_code)]
            #[allow(dead_code)]
            #[inline(always)]
            pub const fn [<try_with_ $field_name>](self, val: $field_type) -> Result<Self, $crate::BitstructError> {
                 if (val as u128) > Self::[<$field_name:upper _MASK>] as u128 { return Err($crate::BitstructError::Overflow { value: val as u128, allocated_bits: Self::[<$field_name:upper _BITS>] }); }
                 Ok(self.[<with_ $field_name>](val))
            }
        }
        $crate::bytestruct!(@impl_typed_fields $name, $unit, $prim, $shift + $crate::bits_mul!($unit, $count), $($rest)*);
    };

    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident ($field_type:ty) = @unroll_signed($count:tt) $($rest:tt)*) => {
        $crate::paste::paste! {
            #[allow(dead_code)]
            pub const [<$field_name:upper _OFFSET>]: usize = $shift;
            #[allow(dead_code)]
            pub const [<$field_name:upper _BITS>]: usize = $crate::bits_mul!($unit, $count);
            #[allow(dead_code)]
            #[doc(hidden)]
            const [<$field_name:upper _MASK>]: $field_type = (!0 as $field_type) >> (<$field_type as $crate::BitLength>::BITS - Self::[<$field_name:upper _BITS>]);

            #[allow(dead_code)]
            #[doc(hidden)]
            const [<$field_name:upper _SHIFT_UP>]: usize = <$field_type as $crate::BitLength>::BITS - Self::[<$field_name:upper _BITS>];

            #[allow(dead_code)]
            #[inline(always)]
            pub const fn $field_name(self) -> $field_type {
                 let val = $crate::bytestruct!(@unroll_typed_read self.0, $unit, $prim, 0, $count, 0, Self::[<$field_name:upper _BITS>]);
                 let mut signed_val = val as $field_type;
                 signed_val = (signed_val << Self::[<$field_name:upper _SHIFT_UP>]) >> Self::[<$field_name:upper _SHIFT_UP>];
                 signed_val
            }
            #[allow(dead_code)]
            #[inline(always)]
            pub fn [<set_ $field_name>](&mut self, val: $field_type) {
                 debug_assert!(
                     (val >= ((!0 as $field_type) << (Self::[<$field_name:upper _BITS>] - 1))) &&
                     (val <= !((!0 as $field_type) << (Self::[<$field_name:upper _BITS>] - 1))),
                     "Value overflows allocated bits"
                 );
                 // Cast to unsigned for underlying bitwise ops
                 let unsigned_val = val as $prim;
                 $crate::bytestruct!(@unroll_typed_write self.0, $unit, $prim, 0, $count, 0, Self::[<$field_name:upper _BITS>], unsigned_val);
            }
            #[allow(dead_code)]
            #[inline(always)]
            pub const fn [<with_ $field_name>](mut self, val: $field_type) -> Self {
                 debug_assert!(
                     (val >= ((!0 as $field_type) << (Self::[<$field_name:upper _BITS>] - 1))) &&
                     (val <= !((!0 as $field_type) << (Self::[<$field_name:upper _BITS>] - 1))),
                     "Value overflows allocated bits"
                 );
                 let unsigned_val = val as $prim;
                 $crate::bytestruct!(@unroll_typed_write self.0, $unit, $prim, 0, $count, 0, Self::[<$field_name:upper _BITS>], unsigned_val);
                 self
            }
            #[allow(dead_code)]
            #[inline(always)]
            pub fn [<try_set_ $field_name>](&mut self, val: $field_type) -> Result<(), $crate::BitstructError> {
                 let min = (!0 as $field_type) << (Self::[<$field_name:upper _BITS>] - 1);
                 let max = !min;
                 if val < min || val > max { return Err($crate::BitstructError::Overflow { value: (val as i128) as u128, allocated_bits: Self::[<$field_name:upper _BITS>] }); }
                 self.[<set_ $field_name>](val); Ok(())
            }
            #[allow(dead_code)]
            #[inline(always)]
            pub const fn [<try_with_ $field_name>](self, val: $field_type) -> Result<Self, $crate::BitstructError> {
                 let min = (!0 as $field_type) << (Self::[<$field_name:upper _BITS>] - 1);
                 let max = !min;
                 if val < min || val > max { return Err($crate::BitstructError::Overflow { value: (val as i128) as u128, allocated_bits: Self::[<$field_name:upper _BITS>] }); }
                 Ok(self.[<with_ $field_name>](val))
            }
        }
        $crate::bytestruct!(@impl_typed_fields $name, $unit, $prim, $shift + $crate::bits_mul!($unit, $count), $($rest)*);
    };

    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, ) => {};
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident bool $bits:tt $($rest:tt)*) => {
        $crate::paste::paste! {
            #[allow(dead_code)]
            /// The bit-offset of the `$field_name` property within the underlying storage.
            pub const [<$field_name:upper _OFFSET>]: usize = $shift;
            #[allow(dead_code)]
            /// The number of bits allocated for the `$field_name` property.
            pub const [<$field_name:upper _BITS>]: usize = $bits;
            #[allow(dead_code)]
            #[doc(hidden)]
            const [<$field_name:upper _MASK>]: $prim = (!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - Self::[<$field_name:upper _BITS>]);
            #[allow(dead_code)]
            $field_vis const fn $field_name(self) -> bool { $crate::bytestruct!(@read_typed_prim self.0, $unit, $prim, $shift, $bits) != 0 }
            #[allow(dead_code)]
            $field_vis fn [<set_ $field_name>](&mut self, val: bool) { $crate::bytestruct!(@write_typed_prim self.0, $unit, $prim, $shift, $bits, val as $prim); }
            #[allow(dead_code)]
            $field_vis const fn [<with_ $field_name>](mut self, val: bool) -> Self { $crate::bytestruct!(@write_typed_prim self.0, $unit, $prim, $shift, $bits, val as $prim); self }
            #[allow(dead_code)]
            $field_vis fn [<try_set_ $field_name>](&mut self, val: bool) -> Result<(), $crate::BitstructError> { self.[<set_ $field_name>](val); Ok(()) }
            #[allow(dead_code)]
            $field_vis const fn [<try_with_ $field_name>](self, val: bool) -> Result<Self, $crate::BitstructError> { Ok(self.[<with_ $field_name>](val)) }
        }
        $crate::bytestruct!(@impl_typed_fields $name, $unit, $prim, $shift + $bits, $($rest)*);
    };

    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident u8 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_typed_int $name, $unit, $prim, $shift, $field_vis $field_name u8 $bits $($rest)*); };
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident u16 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_typed_int $name, $unit, $prim, $shift, $field_vis $field_name u16 $bits $($rest)*); };
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident u32 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_typed_int $name, $unit, $prim, $shift, $field_vis $field_name u32 $bits $($rest)*); };
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident u64 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_typed_int $name, $unit, $prim, $shift, $field_vis $field_name u64 $bits $($rest)*); };
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident u128 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_typed_int $name, $unit, $prim, $shift, $field_vis $field_name u128 $bits $($rest)*); };
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident i8 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_typed_signed_int $name, $unit, $prim, $shift, $field_vis $field_name i8 $bits $($rest)*); };
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident i16 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_typed_signed_int $name, $unit, $prim, $shift, $field_vis $field_name i16 $bits $($rest)*); };
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident i32 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_typed_signed_int $name, $unit, $prim, $shift, $field_vis $field_name i32 $bits $($rest)*); };
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident i64 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_typed_signed_int $name, $unit, $prim, $shift, $field_vis $field_name i64 $bits $($rest)*); };
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident i128 $bits:tt $($rest:tt)*) => { $crate::bytestruct!(@impl_typed_signed_int $name, $unit, $prim, $shift, $field_vis $field_name i128 $bits $($rest)*); };

    (@impl_typed_int $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident $field_type:tt $bits:tt $($rest:tt)*) => {
        $crate::paste::paste! {
            #[allow(dead_code)]
            /// The bit-offset of the `$field_name` property within the underlying storage.
            pub const [<$field_name:upper _OFFSET>]: usize = $shift;
            #[allow(dead_code)]
            /// The number of bits allocated for the `$field_name` property.
            pub const [<$field_name:upper _BITS>]: usize = $bits;
            #[allow(dead_code)]
            #[doc(hidden)]
            const [<$field_name:upper _MASK>]: $prim = (!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - Self::[<$field_name:upper _BITS>]);
            #[allow(dead_code)]
            $field_vis const fn $field_name(self) -> $field_type { $crate::bytestruct!(@read_typed_prim self.0, $unit, $prim, $shift, $bits) as $field_type }
            #[allow(dead_code)]
            $field_vis fn [<set_ $field_name>](&mut self, val: $field_type) {
                debug_assert!((val as u128) <= Self::[<$field_name:upper _MASK>] as u128, "Value {} overflows allocated {} bits", val, $bits);
                $crate::bytestruct!(@write_typed_prim self.0, $unit, $prim, $shift, $bits, val as $prim);
            }
            #[allow(dead_code)]
            $field_vis const fn [<with_ $field_name>](mut self, val: $field_type) -> Self {
                debug_assert!((val as u128) <= Self::[<$field_name:upper _MASK>] as u128, "Value overflows allocated bits");
                $crate::bytestruct!(@write_typed_prim self.0, $unit, $prim, $shift, $bits, val as $prim);
                self
            }
            #[allow(dead_code)]
            $field_vis fn [<try_set_ $field_name>](&mut self, val: $field_type) -> Result<(), $crate::BitstructError> {
                if (val as u128) > Self::[<$field_name:upper _MASK>] as u128 { return Err($crate::BitstructError::Overflow { value: val as u128, allocated_bits: $bits }); }
                self.[<set_ $field_name>](val); Ok(())
            }
            #[allow(dead_code)]
            $field_vis const fn [<try_with_ $field_name>](self, val: $field_type) -> Result<Self, $crate::BitstructError> {
                if (val as u128) > Self::[<$field_name:upper _MASK>] as u128 { return Err($crate::BitstructError::Overflow { value: val as u128, allocated_bits: $bits }); }
                Ok(self.[<with_ $field_name>](val))
            }
        }
        $crate::bytestruct!(@impl_typed_fields $name, $unit, $prim, $shift + $bits, $($rest)*);
    };

    // Signed integer implementation for explicit array-typed `bytestruct!` fields (`[u16; N]`, `[u32; N]`, etc.).
    // Similar to the standard signed implementation, but operates on custom typed sub-primitives,
    // extracting sub-arrays and re-mapping them linearly before applying the sign propagation.
    (@impl_typed_signed_int $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident $field_type:tt $bits:tt $($rest:tt)*) => {
        $crate::paste::paste! {
            #[allow(dead_code)]
            /// The bit-offset of the `$field_name` property within the underlying storage.
            pub const [<$field_name:upper _OFFSET>]: usize = $shift;
            #[allow(dead_code)]
            /// The number of bits allocated for the `$field_name` property.
            pub const [<$field_name:upper _BITS>]: usize = $bits;
            #[allow(dead_code)]
            #[doc(hidden)]
            const [<$field_name:upper _MASK>]: $prim = (!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - Self::[<$field_name:upper _BITS>]);
            #[allow(dead_code)]
            #[doc(hidden)]
            pub const [<$field_name:upper _MIN>]: $field_type = (!0 as $field_type) << (Self::[<$field_name:upper _BITS>] - 1);
            #[allow(dead_code)]
            #[doc(hidden)]
            pub const [<$field_name:upper _MAX>]: $field_type = !Self::[<$field_name:upper _MIN>];
            #[allow(dead_code)]
            #[doc(hidden)]
            const [<$field_name:upper _SHIFT_UP>]: usize = <$field_type as $crate::BitLength>::BITS - Self::[<$field_name:upper _BITS>];

            #[allow(dead_code)]
            $field_vis const fn $field_name(self) -> $field_type {
                let raw = $crate::bytestruct!(@read_typed_prim self.0, $unit, $prim, $shift, $bits) as $field_type;
                (raw << Self::[<$field_name:upper _SHIFT_UP>]) >> Self::[<$field_name:upper _SHIFT_UP>]
            }
            #[allow(dead_code)]
            $field_vis fn [<set_ $field_name>](&mut self, val: $field_type) {
                debug_assert!(val >= Self::[<$field_name:upper _MIN>] && val <= Self::[<$field_name:upper _MAX>], "Value {} out of bounds for {} bits", val, $bits);
                $crate::bytestruct!(@write_typed_prim self.0, $unit, $prim, $shift, $bits, val as $prim);
            }
            #[allow(dead_code)]
            $field_vis const fn [<with_ $field_name>](mut self, val: $field_type) -> Self {
                debug_assert!(val >= Self::[<$field_name:upper _MIN>] && val <= Self::[<$field_name:upper _MAX>], "Value overflows allocated bits");
                $crate::bytestruct!(@write_typed_prim self.0, $unit, $prim, $shift, $bits, val as $prim);
                self
            }
            #[allow(dead_code)]
            $field_vis fn [<try_set_ $field_name>](&mut self, val: $field_type) -> Result<(), $crate::BitstructError> {
                if val < Self::[<$field_name:upper _MIN>] || val > Self::[<$field_name:upper _MAX>] { return Err($crate::BitstructError::Overflow { value: val as i128 as u128, allocated_bits: $bits }); }
                self.[<set_ $field_name>](val); Ok(())
            }
            #[allow(dead_code)]
            $field_vis const fn [<try_with_ $field_name>](self, val: $field_type) -> Result<Self, $crate::BitstructError> {
                if val < Self::[<$field_name:upper _MIN>] || val > Self::[<$field_name:upper _MAX>] { return Err($crate::BitstructError::Overflow { value: val as i128 as u128, allocated_bits: $bits }); }
                Ok(self.[<with_ $field_name>](val))
            }
        }
        $crate::bytestruct!(@impl_typed_fields $name, $unit, $prim, $shift + $bits, $($rest)*);
    };

    // Wrapped Type arm
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident ($field_type:ty) $bits:tt $($rest:tt)*) => {
        $crate::paste::paste! {
            #[allow(dead_code)]
            /// The bit-offset of the `$field_name` property within the underlying storage.
            pub const [<$field_name:upper _OFFSET>]: usize = $shift;
            #[allow(dead_code)]
            /// The number of bits allocated for the `$field_name` property.
            pub const [<$field_name:upper _BITS>]: usize = $bits;
            #[allow(dead_code)]
            #[doc(hidden)]
            const [<$field_name:upper _MASK>]: $prim = (!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - Self::[<$field_name:upper _BITS>]);
            #[allow(dead_code)]
            $field_vis const fn $field_name(self) -> $field_type { $crate::bytestruct!(@read_typed_prim self.0, $unit, $prim, $shift, $bits) as $field_type }
            #[allow(dead_code)]
            $field_vis fn [<set_ $field_name>](&mut self, val: $field_type) { $crate::bytestruct!(@write_typed_prim self.0, $unit, $prim, $shift, $bits, val as $prim); }
            #[allow(dead_code)]
            $field_vis const fn [<with_ $field_name>](mut self, val: $field_type) -> Self { $crate::bytestruct!(@write_typed_prim self.0, $unit, $prim, $shift, $bits, val as $prim); self }
            #[allow(dead_code)]
            $field_vis fn [<try_set_ $field_name>](&mut self, val: $field_type) -> Result<(), $crate::BitstructError> {
                if (val as u128) > Self::[<$field_name:upper _MASK>] as u128 { return Err($crate::BitstructError::Overflow { value: val as u128, allocated_bits: $bits }); }
                self.[<set_ $field_name>](val); Ok(())
            }
            #[allow(dead_code)]
            $field_vis const fn [<try_with_ $field_name>](self, val: $field_type) -> Result<Self, $crate::BitstructError> {
                if (val as u128) > Self::[<$field_name:upper _MASK>] as u128 { return Err($crate::BitstructError::Overflow { value: val as u128, allocated_bits: $bits }); }
                Ok(self.[<with_ $field_name>](val))
            }
        }
        $crate::bytestruct!(@impl_typed_fields $name, $unit, $prim, $shift + $bits, $($rest)*);
    };

    // Enum arm
    (@impl_typed_fields $name:ident, $unit:tt, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident $field_type:tt $bits:tt $($rest:tt)*) => {
        $crate::paste::paste! {
            #[allow(dead_code)]
            /// The bit-offset of the `$field_name` property within the underlying storage.
            pub const [<$field_name:upper _OFFSET>]: usize = $shift;
            #[allow(dead_code)]
            /// The number of bits allocated for the `$field_name` property.
            pub const [<$field_name:upper _BITS>]: usize = $bits;
            #[allow(dead_code)]
            #[doc(hidden)]
            const [<$field_name:upper _MASK>]: $prim = (!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - Self::[<$field_name:upper _BITS>]);
            #[allow(dead_code)]
            $field_vis const fn $field_name(self) -> $field_type { $field_type::from_bits($crate::bytestruct!(@read_typed_prim self.0, $unit, $prim, $shift, $bits) as _) }
            #[allow(dead_code)]
            $field_vis fn [<set_ $field_name>](&mut self, val: $field_type) {
                const _: () = assert!(<$field_type>::BITS <= $bits, "Enum bit width exceeds allocated field width");
                $crate::bytestruct!(@write_typed_prim self.0, $unit, $prim, $shift, $bits, val.to_bits() as $prim);
            }
            #[allow(dead_code)]
            $field_vis const fn [<with_ $field_name>](mut self, val: $field_type) -> Self {
                const _: () = assert!(<$field_type>::BITS <= $bits, "Enum bit width exceeds allocated field width");
                $crate::bytestruct!(@write_typed_prim self.0, $unit, $prim, $shift, $bits, val.to_bits() as $prim); self
            }
            #[allow(dead_code)]
            $field_vis fn [<try_set_ $field_name>](&mut self, val: $field_type) -> Result<(), $crate::BitstructError> {
                 self.[<set_ $field_name>](val); Ok(())
            }
            #[allow(dead_code)]
            $field_vis const fn [<try_with_ $field_name>](self, val: $field_type) -> Result<Self, $crate::BitstructError> {
                 Ok(self.[<with_ $field_name>](val))
            }
        }
        $crate::bytestruct!(@impl_typed_fields $name, $unit, $prim, $shift + $bits, $($rest)*);
    };

    // --- FIELD ACCESSOR ROUTING ---
    // These helpers resolve logical bit shifts/masks into physical array indices and bit offsets.
    // They then route the operation to the fully-unrolled bitwise matching logic.

    (@read_typed_prim $arr:expr, $unit:tt, $prim:ty, $shift:expr, $bits:tt) => {
        {
            const UNIT_BITS: usize = <$unit as $crate::BitLength>::BITS;
            // Physical array index where the field begins
            const WORD_INDEX: usize = $shift / UNIT_BITS;
            // Bit offset within that initial array element
            const BIT_OFFSET: usize = $shift % UNIT_BITS;
            // Number of array elements spanned by this field
            const WORD_COUNT: usize = ($shift as usize + $bits as usize).div_ceil(UNIT_BITS) - WORD_INDEX;

            // Route to the unrolling engine
            $crate::bytestruct!(@unroll_typed_read $arr, $unit, $prim, WORD_INDEX, WORD_COUNT, BIT_OFFSET, $bits)
        }
    };

    (@write_typed_prim $arr:expr, $unit:tt, $prim:ty, $shift:expr, $bits:tt, $val:expr) => {
        {
            const UNIT_BITS: usize = <$unit as $crate::BitLength>::BITS;
            // Physical array index where the field begins
            const WORD_INDEX: usize = $shift / UNIT_BITS;
            // Bit offset within that initial array element
            const BIT_OFFSET: usize = $shift % UNIT_BITS;
            // Number of array elements spanned by this field
            const WORD_COUNT: usize = ($shift as usize + $bits as usize).div_ceil(UNIT_BITS) - WORD_INDEX;

            // Route to the unrolling engine
            $crate::bytestruct!(@unroll_typed_write $arr, $unit, $prim, WORD_INDEX, WORD_COUNT, BIT_OFFSET, $bits, $val)
        }
    };

    // --- UNROLLING ENGINE ---
    // This engine provides literal bitwise expressions for various element counts (1-8).
    // By using literal branches instead of loops, LLVM can perfectly optimize the operation
    // into a single contiguous load/mask/shift sequence (Instruction Fusion).
    //
    // Note: The $prim type is dynamically selected (u32, u64, u128) based on total bit width
    // to minimize register pressure on 32-bit and 64-bit architectures.
    (@unroll_typed_read $arr:expr, $unit:tt, $prim:ty, $idx:expr, 1, $offset:expr, $bits:expr) => {
        (($arr[$idx] as $prim >> $offset) & ((!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - $bits)))
    };
    (@unroll_typed_read $arr:expr, $unit:tt, $prim:ty, $idx:expr, 2, $offset:expr, $bits:expr) => {
        ((($arr[$idx] as $prim | (($arr[$idx+1] as $prim) << $crate::bits_mul!($unit, 1))) >> $offset) & ((!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - $bits)))
    };
    (@unroll_typed_read $arr:expr, $unit:tt, $prim:ty, $idx:expr, 3, $offset:expr, $bits:expr) => {
        ((($arr[$idx] as $prim | (($arr[$idx+1] as $prim) << $crate::bits_mul!($unit, 1)) | (($arr[$idx+2] as $prim) << $crate::bits_mul!($unit, 2))) >> $offset) & ((!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - $bits)))
    };
    (@unroll_typed_read $arr:expr, $unit:tt, $prim:ty, $idx:expr, 4, $offset:expr, $bits:expr) => {
        ((($arr[$idx] as $prim | (($arr[$idx+1] as $prim) << $crate::bits_mul!($unit, 1)) | (($arr[$idx+2] as $prim) << $crate::bits_mul!($unit, 2)) | (($arr[$idx+3] as $prim) << $crate::bits_mul!($unit, 3))) >> $offset) & ((!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - $bits)))
    };
    (@unroll_typed_read $arr:expr, $unit:tt, $prim:ty, $idx:expr, 5, $offset:expr, $bits:expr) => {
        ((($arr[$idx] as $prim | (($arr[$idx+1] as $prim) << $crate::bits_mul!($unit, 1)) | (($arr[$idx+2] as $prim) << $crate::bits_mul!($unit, 2)) | (($arr[$idx+3] as $prim) << $crate::bits_mul!($unit, 3)) | (($arr[$idx+4] as $prim) << $crate::bits_mul!($unit, 4))) >> $offset) & ((!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - $bits)))
    };
    (@unroll_typed_read $arr:expr, $unit:tt, $prim:ty, $idx:expr, 6, $offset:expr, $bits:expr) => {
        ((($arr[$idx] as $prim | (($arr[$idx+1] as $prim) << $crate::bits_mul!($unit, 1)) | (($arr[$idx+2] as $prim) << $crate::bits_mul!($unit, 2)) | (($arr[$idx+3] as $prim) << $crate::bits_mul!($unit, 3)) | (($arr[$idx+4] as $prim) << $crate::bits_mul!($unit, 4)) | (($arr[$idx+5] as $prim) << $crate::bits_mul!($unit, 5))) >> $offset) & ((!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - $bits)))
    };
    (@unroll_typed_read $arr:expr, $unit:tt, $prim:ty, $idx:expr, 7, $offset:expr, $bits:expr) => {
        ((($arr[$idx] as $prim | (($arr[$idx+1] as $prim) << $crate::bits_mul!($unit, 1)) | (($arr[$idx+2] as $prim) << $crate::bits_mul!($unit, 2)) | (($arr[$idx+3] as $prim) << $crate::bits_mul!($unit, 3)) | (($arr[$idx+4] as $prim) << $crate::bits_mul!($unit, 4)) | (($arr[$idx+5] as $prim) << $crate::bits_mul!($unit, 5)) | (($arr[$idx+6] as $prim) << $crate::bits_mul!($unit, 6))) >> $offset) & ((!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - $bits)))
    };
    (@unroll_typed_read $arr:expr, $unit:tt, $prim:ty, $idx:expr, 8, $offset:expr, $bits:expr) => {
        ((($arr[$idx] as $prim | (($arr[$idx+1] as $prim) << $crate::bits_mul!($unit, 1)) | (($arr[$idx+2] as $prim) << $crate::bits_mul!($unit, 2)) | (($arr[$idx+3] as $prim) << $crate::bits_mul!($unit, 3)) | (($arr[$idx+4] as $prim) << $crate::bits_mul!($unit, 4)) | (($arr[$idx+5] as $prim) << $crate::bits_mul!($unit, 5)) | (($arr[$idx+6] as $prim) << $crate::bits_mul!($unit, 6)) | (($arr[$idx+7] as $prim) << $crate::bits_mul!($unit, 7))) >> $offset) & ((!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - $bits)))
    };
    (@unroll_typed_read $arr:expr, $unit:tt, $prim:ty, $idx:expr, $any:expr, $offset:expr, $bits:expr) => {
        // Fallback for large or complex counts
        {
             let mut i = 0;
             let mut acc = 0 as $prim;
             let mut shift = 0;
             while i < $any {
                 acc |= ($arr[$idx + i] as $prim) << shift;
                 shift += <$unit as $crate::BitLength>::BITS;
                 i += 1;
             }
             ((acc >> $offset) & ((!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - $bits)))
        }
    };

    (@unroll_typed_write $arr:expr, $unit:tt, $prim:ty, $idx:expr, $count:tt, $offset:expr, $bits:expr, $val:expr) => {
        {
            let mut full = $crate::bytestruct!(@unroll_typed_read $arr, $unit, $prim, $idx, $count, 0, <$prim as $crate::BitLength>::BITS);
            let mask = (!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - $bits);
            full = (full & !(mask << $offset)) | (($val & mask) << $offset);
            $crate::bytestruct!(@unroll_typed_write_raw $arr, $unit, $prim, $idx, $count, full);
        }
    };

    (@unroll_typed_write_fresh $arr:expr, $unit:tt, $prim:ty, $idx:expr, $count:tt, $offset:expr, $bits:expr, $val:expr) => {
        {
            let mask = (!0 as $prim) >> (<$prim as $crate::BitLength>::BITS - $bits);
            let full = ($val & mask) << $offset;
            $crate::bytestruct!(@unroll_typed_write_raw $arr, $unit, $prim, $idx, $count, full);
        }
    };

    (@unroll_typed_write_raw $arr:expr, $unit:tt, $prim:ty, $idx:expr, 1, $full:expr) => {
        $arr[$idx] = $full as $unit;
    };
    (@unroll_typed_write_raw $arr:expr, $unit:tt, $prim:ty, $idx:expr, 2, $full:expr) => {
        $arr[$idx] = $full as $unit;
        $arr[$idx+1] = ($full >> $crate::bits_mul!($unit, 1)) as $unit;
    };
    (@unroll_typed_write_raw $arr:expr, $unit:tt, $prim:ty, $idx:expr, 3, $full:expr) => {
        $arr[$idx] = $full as $unit;
        $arr[$idx+1] = ($full >> $crate::bits_mul!($unit, 1)) as $unit;
        $arr[$idx+2] = ($full >> $crate::bits_mul!($unit, 2)) as $unit;
    };
    (@unroll_typed_write_raw $arr:expr, $unit:tt, $prim:ty, $idx:expr, 4, $full:expr) => {
        $arr[$idx] = $full as $unit;
        $arr[$idx+1] = ($full >> $crate::bits_mul!($unit, 1)) as $unit;
        $arr[$idx+2] = ($full >> $crate::bits_mul!($unit, 2)) as $unit;
        $arr[$idx+3] = ($full >> $crate::bits_mul!($unit, 3)) as $unit;
    };
    (@unroll_typed_write_raw $arr:expr, $unit:tt, $prim:ty, $idx:expr, 5, $full:expr) => {
        $arr[$idx] = $full as $unit;
        $arr[$idx+1] = ($full >> $crate::bits_mul!($unit, 1)) as $unit;
        $arr[$idx+2] = ($full >> $crate::bits_mul!($unit, 2)) as $unit;
        $arr[$idx+3] = ($full >> $crate::bits_mul!($unit, 3)) as $unit;
        $arr[$idx+4] = ($full >> $crate::bits_mul!($unit, 4)) as $unit;
    };
    (@unroll_typed_write_raw $arr:expr, $unit:tt, $prim:ty, $idx:expr, 6, $full:expr) => {
        $arr[$idx] = $full as $unit;
        $arr[$idx+1] = ($full >> $crate::bits_mul!($unit, 1)) as $unit;
        $arr[$idx+2] = ($full >> $crate::bits_mul!($unit, 2)) as $unit;
        $arr[$idx+3] = ($full >> $crate::bits_mul!($unit, 3)) as $unit;
        $arr[$idx+4] = ($full >> $crate::bits_mul!($unit, 4)) as $unit;
        $arr[$idx+5] = ($full >> $crate::bits_mul!($unit, 5)) as $unit;
    };
    (@unroll_typed_write_raw $arr:expr, $unit:tt, $prim:ty, $idx:expr, 7, $full:expr) => {
        $arr[$idx] = $full as $unit;
        $arr[$idx+1] = ($full >> $crate::bits_mul!($unit, 1)) as $unit;
        $arr[$idx+2] = ($full >> $crate::bits_mul!($unit, 2)) as $unit;
        $arr[$idx+3] = ($full >> $crate::bits_mul!($unit, 3)) as $unit;
        $arr[$idx+4] = ($full >> $crate::bits_mul!($unit, 4)) as $unit;
        $arr[$idx+5] = ($full >> $crate::bits_mul!($unit, 5)) as $unit;
        $arr[$idx+6] = ($full >> $crate::bits_mul!($unit, 6)) as $unit;
    };
    (@unroll_typed_write_raw $arr:expr, $unit:tt, $prim:ty, $idx:expr, 8, $full:expr) => {
        $arr[$idx] = $full as $unit;
        $arr[$idx+1] = ($full >> $crate::bits_mul!($unit, 1)) as $unit;
        $arr[$idx+2] = ($full >> $crate::bits_mul!($unit, 2)) as $unit;
        $arr[$idx+3] = ($full >> $crate::bits_mul!($unit, 3)) as $unit;
        $arr[$idx+4] = ($full >> $crate::bits_mul!($unit, 4)) as $unit;
        $arr[$idx+5] = ($full >> $crate::bits_mul!($unit, 5)) as $unit;
        $arr[$idx+6] = ($full >> $crate::bits_mul!($unit, 6)) as $unit;
        $arr[$idx+7] = ($full >> $crate::bits_mul!($unit, 7)) as $unit;
    };
    (@unroll_typed_write_raw $arr:expr, $unit:tt, $prim:ty, $idx:expr, $any:expr, $full:expr) => {
        {
             let mut i = 0;
             let mut shift = 0;
             while i < $any {
                 $arr[$idx + i] = ($full >> shift) as $unit;
                 shift += <$unit as $crate::BitLength>::BITS;
                 i += 1;
             }
        }
    };
    // Wrapped Type arm (for byteval expansion or trait-associated types)
    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident ($field_type:ty) $bits:tt $($rest:tt)*) => {
        $crate::paste::paste! {
            const [<$field_name:upper _OFFSET>]: usize = $shift;
            const [<$field_name:upper _BITS>]: usize = $bits;
            const [<$field_name:upper _MASK>]: u128 = (!0u128) >> (128 - $bits);

            #[allow(dead_code)]
            #[inline]
            $field_vis const fn $field_name(self) -> $field_type {
                let val = $crate::bytestruct!(@read_localized_prim self.0, $shift, $bits);
                val as $field_type
            }

            #[allow(dead_code)]
            $field_vis fn [<set_ $field_name>](&mut self, val: $field_type) {
                debug_assert!((val as u128) <= Self::[<$field_name:upper _MASK>], "Value {} overflows allocated {} bits", val, $bits);
                $crate::bytestruct!(@write_localized_prim self.0, $shift, $bits, val as u128);
            }

            #[allow(dead_code)]
            $field_vis const fn [<with_ $field_name>](mut self, val: $field_type) -> Self {
                debug_assert!((val as u128) <= Self::[<$field_name:upper _MASK>], "Value overflows allocated bits");
                $crate::bytestruct!(@write_localized_prim self.0, $shift, $bits, val as u128);
                self
            }

            #[allow(dead_code)]
            $field_vis fn [<try_set_ $field_name>](&mut self, val: $field_type) -> Result<(), $crate::BitstructError> {
                if (val as u128) > Self::[<$field_name:upper _MASK>] {
                    return Err($crate::BitstructError::Overflow { value: val as u128, allocated_bits: $bits });
                }
                self.[<set_ $field_name>](val);
                Ok(())
            }

            #[allow(dead_code)]
            $field_vis const fn [<try_with_ $field_name>](self, val: $field_type) -> Result<Self, $crate::BitstructError> {
                if (val as u128) > Self::[<$field_name:upper _MASK>] {
                    return Err($crate::BitstructError::Overflow { value: val as u128, allocated_bits: $bits });
                }
                Ok(self.[<with_ $field_name>](val))
            }
        }
        $crate::bytestruct!(@impl_fields $name, $prim, $shift + $bits, $($rest)*);
    };

    // Enum arm
    (@impl_fields $name:ident, $prim:ty, $shift:expr, $field_vis:vis $field_name:ident $field_type:tt $bits:tt $($rest:tt)*) => {
        $crate::paste::paste! {
            const [<$field_name:upper _OFFSET>]: usize = $shift;
            const [<$field_name:upper _BITS>]: usize = $bits;
            const [<$field_name:upper _MASK>]: u128 = (!0u128) >> (128 - $bits);

            #[allow(dead_code)]
            #[doc = concat!("Returns the `", stringify!($field_name), "` variant strictly typed to the `", stringify!($field_type), "` enumeration.")]
            $field_vis const fn $field_name(self) -> $field_type {
                let val = $crate::bytestruct!(@read_localized_prim self.0, $shift, $bits);
                $field_type::from_bits(val as _)
            }

            #[allow(dead_code)]
            #[doc = concat!("Inline mutation to apply the bounded `", stringify!($field_type), "` enumeration to the `", stringify!($field_name), "` property.")]
            $field_vis fn [<set_ $field_name>](&mut self, val: $field_type) {
                const _: () = assert!(<$field_type>::BITS <= $bits, "Enum bit width exceeds allocated field width");
                $crate::bytestruct!(@write_localized_prim self.0, $shift, $bits, val.to_bits() as u128);
            }

            #[allow(dead_code)]
            #[doc = concat!("Returns a cloned copy of the bytestruct bounded by the `", stringify!($field_type), "` enumeration supplied to `", stringify!($field_name), "`.")]
            $field_vis const fn [<with_ $field_name>](mut self, val: $field_type) -> Self {
                const _: () = assert!(<$field_type>::BITS <= $bits, "Enum bit width exceeds allocated field width");
                $crate::bytestruct!(@write_localized_prim self.0, $shift, $bits, val.to_bits() as u128);
                self
            }

            #[allow(dead_code)]
            #[doc = concat!("Strict inline mutation to apply the bounded `", stringify!($field_type), "` enumeration to the `", stringify!($field_name), "` property. Returns a `BitstructError` if the value overflows ", stringify!($bits), " bits.")]
            $field_vis fn [<try_set_ $field_name>](&mut self, val: $field_type) -> Result<(), $crate::BitstructError> {
                 self.[<set_ $field_name>](val);
                 Ok(())
            }

            #[allow(dead_code)]
            #[doc = concat!("Strict cloned evaluation to apply the bounded `", stringify!($field_type), "` enumeration to the `", stringify!($field_name), "` property. Returns a `BitstructError` if the value overflows ", stringify!($bits), " bits.")]
            $field_vis const fn [<try_with_ $field_name>](self, val: $field_type) -> Result<Self, $crate::BitstructError> {
                 Ok(self.[<with_ $field_name>](val))
            }
        }
        $crate::bytestruct!(@impl_fields $name, $prim, $shift + $bits, $($rest)*);
    };
    // --- LOCALIZED PRIMITIVE HELPERS ---
    //
    // These helpers implement the "Literal Guard" pattern.
    // Instead of a dynamic loop (which the compiler often fails to unroll for variable-width bitfields),
    // we use a sequence of literal index checks.
    //
    // Since $bits and $shift are constants provided during macro expansion,
    // LLVM can perfectly constant-fold these `if len > N` branches, leaving
    // only a flat sequence of bitwise operations.
    //
    // We specialize in 64-bit registers (u64) for all spans <= 8 bytes to reduce register pressure.

    (@read_localized_prim $arr:expr, $shift:expr, $bits:tt) => {
        {
            const BYTE_INDEX: usize = $shift >> 3;
            const BIT_OFFSET: usize = $shift & 7;
            const BYTE_COUNT: usize = (($shift + $bits + 7) >> 3) - BYTE_INDEX;
            $crate::read_le_bits::<{$shift}, {$bits}, _, BYTE_INDEX, BIT_OFFSET, BYTE_COUNT>(&$arr)
        }
    };

    (@write_localized_prim $arr:expr, $shift:expr, $bits:tt, $val:expr) => {
        {
            const BYTE_INDEX: usize = $shift >> 3;
            const BIT_OFFSET: usize = $shift & 7;
            const BYTE_COUNT: usize = (($shift + $bits + 7) >> 3) - BYTE_INDEX;
            $crate::write_le_bits::<{$shift}, {$bits}, _, BYTE_INDEX, BIT_OFFSET, BYTE_COUNT>(&mut $arr, $val)
        }
    };
}
