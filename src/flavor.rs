use jomini::{BinaryFlavor, Encoding, Utf8Encoding};

pub struct ImperatorFlavor;

impl Encoding for ImperatorFlavor {
    fn decode<'a>(&self, data: &'a [u8]) -> std::borrow::Cow<'a, str> {
        Utf8Encoding::decode(data)
    }
}

impl BinaryFlavor for ImperatorFlavor {
    fn visit_f32_1(&self, data: &[u8]) -> f32 {
        f32::from_bits(le_i32(data) as u32)
    }

    fn visit_f32_2(&self, data: &[u8]) -> f32 {
        le_i32(data) as f32 / 100000.0
    }
}

#[inline]
pub(crate) fn le_i32(data: &[u8]) -> i32 {
    let ptr = data.as_ptr() as *const u8 as *const i32;
    unsafe { ::std::ptr::read_unaligned(ptr).to_le() }
}
