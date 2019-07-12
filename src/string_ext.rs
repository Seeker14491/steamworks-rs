use std::string::FromUtf8Error;

pub(crate) trait FromUtf8NulTruncating<T>
where
    Self: Sized,
{
    fn from_utf8_nul_truncating(_: T) -> Result<Self, FromUtf8Error>;
}

impl FromUtf8NulTruncating<Vec<u8>> for String {
    fn from_utf8_nul_truncating(mut vec: Vec<u8>) -> Result<Self, FromUtf8Error> {
        nul_truncate(&mut vec);
        String::from_utf8(vec)
    }
}

impl FromUtf8NulTruncating<&[u8]> for String {
    fn from_utf8_nul_truncating(slice: &[u8]) -> Result<Self, FromUtf8Error> {
        let vec: Vec<u8> = slice.iter().copied().take_while(|x| *x != 0).collect();
        String::from_utf8(vec)
    }
}

impl FromUtf8NulTruncating<&[i8]> for String {
    fn from_utf8_nul_truncating(slice: &[i8]) -> Result<Self, FromUtf8Error> {
        let vec: Vec<u8> = slice
            .iter()
            .map(|x| *x as u8)
            .take_while(|x| *x != 0)
            .collect();
        String::from_utf8(vec)
    }
}

fn nul_truncate(vec: &mut Vec<u8>) {
    let first_nul_idx = vec.iter().position(|x| *x == 0);
    if let Some(i) = first_nul_idx {
        vec.truncate(i);
    }
}

#[test]
fn test_string_from_bytes_with_interior_nul() {
    assert_eq!(String::from_utf8_nul_truncating(&[0_u8][..]).unwrap(), "");
    assert_eq!(
        String::from_utf8_nul_truncating(&[65_u8, 0][..]).unwrap(),
        "A"
    );
    assert_eq!(
        String::from_utf8_nul_truncating(&[65_u8, 66, 0, 67][..]).unwrap(),
        "AB"
    );
    assert_eq!(
        String::from_utf8_nul_truncating(&[65_u8, 66, 0, 67, 0][..]).unwrap(),
        "AB"
    );
}
