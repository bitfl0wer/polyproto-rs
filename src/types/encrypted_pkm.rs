// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use der::asn1::BitString;

use super::spki::{AlgorithmIdentifierOwned, SubjectPublicKeyInfo};
use super::x509_cert::SerialNumber;

#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
/// A private key material structure for storing encrypted private key material on a home server.
///
/// JSON representation:
/// ```json
/// {
///     "serial_number": [41, 12, 3, 123, 4, 3, 11],
///     "key_data": "-----BEGIN[...]",
///     "encryption_algorithm": [1, 2, 840, 113549, 1, 5, 13, 1, 1, 5]
/// }
/// ```
///
/// where:
///
/// - `serial_number`: [SerialNumber], as an array of integers. Must represent a positive integer of up to 20 octets in length.
/// - `key_data`: [PrivateKeyInfo] as a PEM-encoded ASN.1 structure. This is just a
///               [SubjectPublicKeyInfoOwned] structure which stores an encrypted private key in the
///              `subject_public_key` field.
/// - `encryption_algorithm`: [AlgorithmIdentifierOwned], DER encoded as an array of bytes.
pub struct EncryptedPkm {
    pub serial_number: SerialNumber,
    pub key_data: PrivateKeyInfo,
    pub encryption_algorithm: AlgorithmIdentifierOwned,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Private key material with additional information about the private keys' algorithm.
pub struct PrivateKeyInfo {
    pub algorithm: AlgorithmIdentifierOwned,
    pub encrypted_private_key_bitstring: BitString,
}

impl From<SubjectPublicKeyInfo> for PrivateKeyInfo {
    fn from(value: SubjectPublicKeyInfo) -> Self {
        #[allow(clippy::useless_conversion)]
        PrivateKeyInfo {
            algorithm: value.algorithm.clone().into(),
            encrypted_private_key_bitstring: value.subject_public_key.clone(),
        }
    }
}

impl From<PrivateKeyInfo> for SubjectPublicKeyInfo {
    fn from(value: PrivateKeyInfo) -> Self {
        spki::SubjectPublicKeyInfoOwned {
            algorithm: value.algorithm.into(),
            subject_public_key: value.encrypted_private_key_bitstring,
        }
        .into()
    }
}

#[cfg(feature = "serde")]
mod serde_support {
    use der::pem::LineEnding;
    use serde::de::Visitor;
    use serde::{Deserialize, Serialize};

    use crate::types::spki::SubjectPublicKeyInfo;

    use super::PrivateKeyInfo;

    struct PrivateKeyInfoVisitor;

    impl<'de> Visitor<'de> for PrivateKeyInfoVisitor {
        type Value = PrivateKeyInfo;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a private key info structure, which is a subject public key info structure as defined in RFC 5280. this private key info structure needs to be a valid PEM encoded ASN.1 structure")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            SubjectPublicKeyInfo::from_pem(v.as_bytes())
                .map_err(serde::de::Error::custom)
                .map(Into::into)
        }
    }

    impl<'de> Deserialize<'de> for crate::types::encrypted_pkm::PrivateKeyInfo {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_str(PrivateKeyInfoVisitor)
        }
    }

    impl Serialize for crate::types::encrypted_pkm::PrivateKeyInfo {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_str(
                SubjectPublicKeyInfo::from(self.clone())
                    .to_pem(LineEnding::LF)
                    .map_err(serde::ser::Error::custom)?
                    .as_str(),
            )
        }
    }
}
