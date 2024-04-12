// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::str::FromStr;

use der::asn1::{OctetString, SetOfVec};
use der::{Any, Encode, Tag, Tagged};
use spki::ObjectIdentifier;
use x509_cert::attr::Attribute;
use x509_cert::ext::Extension;

use crate::errors::base::InvalidInput;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// The key usage extension defines the purpose of the key contained in the certificate. The usage
/// restriction might be employed when a key that could be used for more than one operation is to
/// be restricted. See <https://cryptography.io/en/latest/x509/reference/#cryptography.x509.KeyUsage>
pub enum KeyUsage {
    /// This purpose is set to true when the subject public key is used for verifying digital
    /// signatures, other than signatures on certificates (`key_cert_sign`) and CRLs (`crl_sign`).
    DigitalSignature(bool),
    /// This purpose is set to true when the subject public key is used for verifying signatures on
    /// certificate revocation lists.
    CrlSign(bool),
    /// This purpose is set to true when the subject public key is used for verifying digital
    /// signatures, other than signatures on certificates (`key_cert_sign`) and CRLs (`crl_sign`).
    /// It is used to provide a non-repudiation service that protects against the signing entity
    /// falsely denying some action. In the case of later conflict, a reliable third party may
    /// determine the authenticity of the signed data. This was called `non_repudiation` in older
    /// revisions of the X.509 specification.
    ContentCommitment(bool),
    /// This purpose is set to true when the subject public key is used for enciphering private or
    /// secret keys.
    KeyEncipherment(bool),
    /// This purpose is set to true when the subject public key is used for directly enciphering raw
    /// user data without the use of an intermediate symmetric cipher.
    DataEncipherment(bool),
    /// This purpose is set to true when the subject public key is used for key agreement. For
    /// example, when a Diffie-Hellman key is to be used for key management, then this purpose is
    /// set to true.
    KeyAgreement(bool),
    /// This purpose is set to true when the subject public key is used for verifying signatures on
    /// public key certificates. If this purpose is set to true then ca must be true in the
    /// `BasicConstraints` extension.
    KeyCertSign(bool),
    /// When this purposes is set to true and the `key_agreement` purpose is also set, the subject
    /// public key may be used only for enciphering data while performing key agreement. The
    /// `KeyAgreement` capability must be set to `true` for this.
    EncipherOnly(bool),
    /// When this purposes is set to true and the `key_agreement` purpose is also set, the subject
    /// public key may be used only for deciphering data while performing key agreement. The
    /// `KeyAgreement` capability must be set to `true` for this.
    DecipherOnly(bool),
}

impl From<KeyUsage> for bool {
    fn from(value: KeyUsage) -> Self {
        match value {
            KeyUsage::DigitalSignature(val) => val,
            KeyUsage::CrlSign(val) => val,
            KeyUsage::ContentCommitment(val) => val,
            KeyUsage::KeyEncipherment(val) => val,
            KeyUsage::DataEncipherment(val) => val,
            KeyUsage::KeyAgreement(val) => val,
            KeyUsage::KeyCertSign(val) => val,
            KeyUsage::EncipherOnly(val) => val,
            KeyUsage::DecipherOnly(val) => val,
        }
    }
}

impl TryFrom<Attribute> for KeyUsage {
    type Error = InvalidInput;

    /// Performs the conversion.
    ///
    /// Fails, if the input attribute does not contain exactly one value, or if the input attribute
    /// does not contain a boolean value. Also fails if the OID of the attribute does not match any
    /// known KeyUsage variant.
    fn try_from(value: Attribute) -> Result<Self, Self::Error> {
        // PRETTYFYME: I know this is a bit of a mess, but it works. If anyone wants to make it
        // prettier, feel free to do so.

        // Check if the attribute contains exactly one value
        if value.values.len() != 1usize {
            return Err(InvalidInput::IncompatibleVariantForConversion { reason: "This attribute does not store exactly one value, as would be expected for a KeyUsage attribute".to_string() });
        }
        let sov = value.values.get(0);

        // The first value inside the Attribute is a SetOfVec. We need to look inside the SetOfVec to
        // find the actual attribute we are interested in.
        if let Some(inner_value) = sov {
            if inner_value.tag() != Tag::Boolean {
                return Err(InvalidInput::IncompatibleVariantForConversion { reason: format!("Only Any objects with boolean tags can be converted to a KeyUsage enum variant. Expected Tag::Boolean, found {:?}", inner_value.tag()) });
            }
            // This is how booleans are apparently encoded in ASN.1
            let boolean_value = match inner_value.value() {
                &[0x00] => false,
                &[0xFF] | &[0x01] => true,
                _ => {
                    return Err(InvalidInput::IncompatibleVariantForConversion {
                        reason: "Encountered unexpected value for Boolean tag".to_string(),
                    });
                }
            };
            // Now we have to match the OID of the attribute to the known KeyUsage variants
            return Ok(match value.oid.to_string().as_str() {
                super::OID_KEY_USAGE_CONTENT_COMMITMENT => KeyUsage::ContentCommitment(boolean_value),
                super::OID_KEY_USAGE_CRL_SIGN => KeyUsage::CrlSign(boolean_value),
                super::OID_KEY_USAGE_DATA_ENCIPHERMENT => KeyUsage::DataEncipherment(boolean_value),
                super::OID_KEY_USAGE_DECIPHER_ONLY => KeyUsage::DecipherOnly(boolean_value),
                super::OID_KEY_USAGE_DIGITAL_SIGNATURE => KeyUsage::DigitalSignature(boolean_value),
                super::OID_KEY_USAGE_ENCIPHER_ONLY => KeyUsage::EncipherOnly(boolean_value),
                super::OID_KEY_USAGE_KEY_AGREEMENT => KeyUsage::KeyAgreement(boolean_value),
                #[allow(unreachable_patterns)] // cargo thinks the below pattern is unreachable.
                super::OID_KEY_USAGE_KEY_CERT_SIGN => KeyUsage::KeyCertSign(boolean_value),
                super::OID_KEY_USAGE_KEY_ENCIPHERMENT => KeyUsage::KeyEncipherment(boolean_value),
                // If the OID does not match any known KeyUsage variant, we return an error
                _ => {
                    return Err(InvalidInput::IncompatibleVariantForConversion {
                            reason: format!("The OID of the attribute does not match any known KeyUsage variant. Found OID \"{}\"", value.oid)
                        },
                    )
                }
            });
        }
        // If the attribute does not contain a value, we return an error
        Err(InvalidInput::IncompatibleVariantForConversion {
            reason: "The attribute does not contain a value".to_string(),
        })
    }
}

impl TryFrom<Extension> for KeyUsage {
    type Error = InvalidInput;

    /// Performs the conversion.
    ///
    /// Fails, if the input attribute does not contain a boolean value. Also fails if the OID of
    /// the attribute does not match any known KeyUsage variant, especially when the unknown OID is
    /// marked as "critical"
    fn try_from(value: Extension) -> Result<Self, Self::Error> {
        #[allow(unreachable_patterns)]
        if value.critical
            && !matches!(
                value.extn_id.to_string().as_str(),
                OID_KEY_USAGE_CONTENT_COMMITMENT
                    | OID_KEY_USAGE_CRL_SIGN
                    | OID_KEY_USAGE_DATA_ENCIPHERMENT
                    | OID_KEY_USAGE_DATA_ENCIPHERMENT
                    | OID_KEY_USAGE_DECIPHER_ONLY
                    | OID_KEY_USAGE_DIGITAL_SIGNATURE
                    | OID_KEY_USAGE_ENCIPHER_ONLY
                    | OID_KEY_USAGE_KEY_AGREEMENT
                    | OID_KEY_USAGE_KEY_CERT_SIGN
                    | OID_KEY_USAGE_KEY_ENCIPHERMENT
            )
        {
            // Error if we encounter a "critical" X.509 extension which we do not know of
            return Err(InvalidInput::UnknownCriticalExtension { oid: value.extn_id });
        }

        let boolean_value = match value.extn_value.as_bytes() {
            &[0x00] => false,
            &[0xFF] | &[0x01] => true,
            _ => {
                return Err(InvalidInput::IncompatibleVariantForConversion {
                    reason: "Encountered unexpected value for Boolean tag".to_string(),
                });
            }
        };
        // Now we have to match the OID of the attribute to the known KeyUsage variants
        #[allow(unreachable_patterns)]
        return Ok(match value.extn_id.to_string().as_str() {
            super::OID_KEY_USAGE_CONTENT_COMMITMENT => KeyUsage::ContentCommitment(boolean_value),
            super::OID_KEY_USAGE_CRL_SIGN => KeyUsage::CrlSign(boolean_value),
            super::OID_KEY_USAGE_DATA_ENCIPHERMENT => KeyUsage::DataEncipherment(boolean_value),
            super::OID_KEY_USAGE_DECIPHER_ONLY => KeyUsage::DecipherOnly(boolean_value),
            super::OID_KEY_USAGE_DIGITAL_SIGNATURE => KeyUsage::DigitalSignature(boolean_value),
            super::OID_KEY_USAGE_ENCIPHER_ONLY => KeyUsage::EncipherOnly(boolean_value),
            super::OID_KEY_USAGE_KEY_AGREEMENT => KeyUsage::KeyAgreement(boolean_value),
            super::OID_KEY_USAGE_KEY_CERT_SIGN => KeyUsage::KeyCertSign(boolean_value),
            super::OID_KEY_USAGE_KEY_ENCIPHERMENT => KeyUsage::KeyEncipherment(boolean_value),
            // If the OID does not match any known KeyUsage variant, we return an error
            _ => {
                return Err(InvalidInput::IncompatibleVariantForConversion {
                        reason: format!("The OID of the attribute does not match any known KeyUsage variant. Found OID \"{}\"", value.extn_id)
                    },
                )
            }
        });
    }
}

impl From<KeyUsage> for Any {
    fn from(value: KeyUsage) -> Self {
        Any::new(der::Tag::Boolean,match bool::from(value) {
            true => vec![0xff],
            false => vec![0x00],
        },
        ).expect("Error occurred when converting BasicConstraints bool to der::Any. Please report this crash at https://github.com/polyphony-chat/polyproto.")
    }
}

impl From<KeyUsage> for ObjectIdentifier {
    fn from(value: KeyUsage) -> Self {
        let result = match value {
            KeyUsage::DigitalSignature(_) => {
                ObjectIdentifier::from_str(super::OID_KEY_USAGE_DIGITAL_SIGNATURE)
            }
            KeyUsage::CrlSign(_) => ObjectIdentifier::from_str(super::OID_KEY_USAGE_CRL_SIGN),
            KeyUsage::ContentCommitment(_) => {
                ObjectIdentifier::from_str(super::OID_KEY_USAGE_CONTENT_COMMITMENT)
            }
            KeyUsage::KeyEncipherment(_) => {
                ObjectIdentifier::from_str(super::OID_KEY_USAGE_KEY_ENCIPHERMENT)
            }
            KeyUsage::DataEncipherment(_) => {
                ObjectIdentifier::from_str(super::OID_KEY_USAGE_DATA_ENCIPHERMENT)
            }
            KeyUsage::KeyAgreement(_) => {
                ObjectIdentifier::from_str(super::OID_KEY_USAGE_KEY_AGREEMENT)
            }
            KeyUsage::KeyCertSign(_) => {
                ObjectIdentifier::from_str(super::OID_KEY_USAGE_KEY_CERT_SIGN)
            }
            KeyUsage::EncipherOnly(_) => {
                ObjectIdentifier::from_str(super::OID_KEY_USAGE_ENCIPHER_ONLY)
            }
            KeyUsage::DecipherOnly(_) => {
                ObjectIdentifier::from_str(super::OID_KEY_USAGE_DECIPHER_ONLY)
            }
        };
        result.expect("Error occurred when converting KeyUsage enum to ObjectIdentifier. Please report this crash at https://github.com/polyphony-chat/polyproto.")
    }
}

impl From<KeyUsage> for Attribute {
    fn from(value: KeyUsage) -> Self {
        // Creating a Any from a bool is really simple, so we can expect this to never fail.
        let any_val = Any::from(value);
        let mut sov = SetOfVec::new();
        // .insert() only fails if the value is not unique. We are inserting a single value, so this
        // should never fail. See tests below for verification.
        sov.insert(any_val).expect("Error occurred when inserting KeyUsage into der::Any to SetOfVec. Please report this crash at https://github.com/polyphony-chat/polyproto");
        Attribute {
            oid: value.into(),
            values: sov,
        }
    }
}

impl From<KeyUsage> for Extension {
    fn from(value: KeyUsage) -> Self {
        Extension {
            extn_id: value.into(),
            critical: true,
            extn_value: OctetString::new(bool::from(value).to_der().expect("Error occured when trying to convert bool to DER. Please report this crash at https://github.com/polyphony-chat/polyproto")).expect("Error occured when trying to convert bool to an OctetString. Please report this crash at https://github.com/polyphony-chat/polyproto"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn key_usage_to_extension() {
        let key_usage = KeyUsage::KeyCertSign(true);
        let extension = Extension::from(key_usage);
        dbg!(extension);
    }
}