// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use der::asn1::Uint;
use der::pem::LineEnding;
use der::{Decode, DecodePem, Encode, EncodePem};
use x509_cert::name::Name;
use x509_cert::time::Validity;
use x509_cert::Certificate;

use crate::errors::ConversionError;
use crate::key::{PrivateKey, PublicKey};
use crate::signature::Signature;
use crate::Constrained;

use super::idcerttbs::IdCertTbs;
use super::idcsr::IdCsr;
use super::Target;

/// A signed polyproto ID-Cert, consisting of the actual certificate, the CA-generated signature and
/// metadata about that signature.
///
/// ID-Certs are valid subset of X.509 v3 certificates. The limitations are documented in the
/// polyproto specification.
///
/// ## Generic Parameters
///
/// - **S**: The [Signature] and - by extension - [SignatureAlgorithm] this certificate was
///   signed with.
/// - **P**: A [PublicKey] type P which can be used to verify [Signature]s of type S.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IdCert<S: Signature, P: PublicKey<S>> {
    /// Inner TBS (To be signed) certificate
    pub id_cert_tbs: IdCertTbs<S, P>,
    /// Signature for the TBS certificate
    pub signature: S,
}

impl<S: Signature, P: PublicKey<S>> IdCert<S, P> {
    /// Create a new [IdCert] by passing an [IdCsr] and other supplementary information. Returns
    /// an error, if the provided IdCsr or issuer [Name] do not pass [Constrained] verification,
    /// i.e. if they are not up to polyproto specification. Also fails if the provided IdCsr has
    /// the [BasicConstraints] "ca" flag set to `false`.
    ///
    /// See [IdCert::from_actor_csr()] when trying to create a new actor certificate.
    ///
    /// The resulting `IdCert` is guaranteed to be well-formed and up to polyproto specification,
    /// for the usage context of a home server certificate.
    pub fn from_ca_csr(
        id_csr: IdCsr<S, P>,
        signing_key: &impl PrivateKey<S, PublicKey = P>,
        serial_number: Uint,
        issuer: Name,
        validity: Validity,
    ) -> Result<Self, ConversionError> {
        let signature_algorithm = signing_key.algorithm_identifier();
        let id_cert_tbs = IdCertTbs::<S, P> {
            serial_number,
            signature_algorithm,
            issuer,
            validity,
            subject: id_csr.inner_csr.subject,
            subject_public_key: id_csr.inner_csr.subject_public_key,
            capabilities: id_csr.inner_csr.capabilities,
            s: std::marker::PhantomData,
        };
        let signature = signing_key.sign(&id_cert_tbs.clone().to_der()?);
        let cert = IdCert {
            id_cert_tbs,
            signature,
        };
        cert.validate(Some(Target::HomeServer))?;
        Ok(cert)
    }

    /// Create a new [IdCert] by passing an [IdCsr] and other supplementary information. Returns
    /// an error, if the provided IdCsr or issuer [Name] do not pass [Constrained] verification,
    /// i.e. if they are not up to polyproto specification. Also fails if the provided IdCsr has
    /// the [BasicConstraints] "ca" flag set to `false`.
    ///
    /// See [IdCert::from_ca_csr()] when trying to create a new ca certificate.
    ///
    /// The resulting `IdCert` is guaranteed to be well-formed and up to polyproto specification,
    /// for the usage context of an actor certificate.
    pub fn from_actor_csr(
        id_csr: IdCsr<S, P>,
        signing_key: &impl PrivateKey<S, PublicKey = P>,
        serial_number: Uint,
        issuer: Name,
        validity: Validity,
    ) -> Result<Self, ConversionError> {
        log::trace!("[IdCert::from_actor_csr()] creating actor certificate");
        let signature_algorithm = signing_key.algorithm_identifier();
        log::trace!("[IdCert::from_actor_csr()] creating IdCertTbs");
        log::trace!("[IdCert::from_actor_csr()] Issuer: {}", issuer.to_string());
        log::trace!(
            "[IdCert::from_actor_csr()] Subject: {}",
            id_csr.inner_csr.subject.to_string()
        );
        let id_cert_tbs = IdCertTbs::<S, P> {
            serial_number,
            signature_algorithm,
            issuer,
            validity,
            subject: id_csr.inner_csr.subject,
            subject_public_key: id_csr.inner_csr.subject_public_key,
            capabilities: id_csr.inner_csr.capabilities,
            s: std::marker::PhantomData,
        };
        log::trace!("[IdCert::from_actor_csr()] creating Signature");
        let signature = signing_key.sign(&id_cert_tbs.clone().to_der()?);
        let cert = IdCert {
            id_cert_tbs,
            signature,
        };
        log::trace!(
            "[IdCert::from_actor_csr()] validating certificate with target {:?}",
            Some(Target::Actor)
        );
        cert.validate(Some(Target::Actor))?;
        Ok(cert)
    }

    /// Create an [IdCert] from a byte slice containing a DER encoded X.509 Certificate.
    /// The resulting `IdCert` is guaranteed to be well-formed and up to polyproto specification,
    /// if the correct [Target] for the certificates' intended usage context is provided.
    pub fn from_der(value: &[u8], target: Option<Target>) -> Result<Self, ConversionError> {
        let cert = IdCert::from_der_unchecked(value)?;
        cert.validate(target)?;
        Ok(cert)
    }

    /// Create an unchecked [IdCert] from a byte slice containing a DER encoded X.509 Certificate.
    /// The caller is responsible for verifying the correctness of this `IdCert` using
    /// the [Constrained] trait before using it.
    pub fn from_der_unchecked(value: &[u8]) -> Result<Self, ConversionError> {
        let cert = IdCert::try_from(Certificate::from_der(value)?)?;
        Ok(cert)
    }

    /// Encode this type as DER, returning a byte vector.
    pub fn to_der(self) -> Result<Vec<u8>, ConversionError> {
        Ok(Certificate::try_from(self)?.to_der()?)
    }

    /// Create an [IdCert] from a byte slice containing a PEM encoded X.509 Certificate.
    /// The resulting `IdCert` is guaranteed to be well-formed and up to polyproto specification,
    /// if the correct [Target] for the certificates' intended usage context is provided.
    pub fn from_pem(pem: &str, target: Option<Target>) -> Result<Self, ConversionError> {
        let cert = IdCert::from_pem_unchecked(pem)?;
        cert.validate(target)?;
        Ok(cert)
    }

    /// Create an unchecked [IdCert] from a byte slice containing a PEM encoded X.509 Certificate.
    /// The caller is responsible for verifying the correctness of this `IdCert` using
    /// the [Constrained] trait before using it.
    pub fn from_pem_unchecked(pem: &str) -> Result<Self, ConversionError> {
        let cert = IdCert::try_from(Certificate::from_pem(pem)?)?;
        Ok(cert)
    }

    /// Encode this type as PEM, returning a string.
    pub fn to_pem(self, line_ending: LineEnding) -> Result<String, ConversionError> {
        Ok(Certificate::try_from(self)?.to_pem(line_ending)?)
    }

    /// Returns a byte vector containing the DER encoded IdCertTbs. This data is encoded
    /// in the signature field of the certificate, and can be used to verify the signature.
    ///
    /// This is a shorthand for `self.id_cert_tbs.clone().to_der()`, since intuitively, one might
    /// try to verify the signature of the certificate by using `self.to_der()`, which will result
    /// in an error.
    pub fn signature_data(&self) -> Result<Vec<u8>, ConversionError> {
        self.id_cert_tbs.clone().to_der()
    }

    /// Performs validation of the certificate. This includes checking the signature, the
    /// validity period, the issuer, subject, [Capabilities] and every other constraint required
    /// by the polyproto specification.
    pub fn valid_at(&self, time: u64, target: Option<Target>) -> bool {
        self.id_cert_tbs.valid_at(time) && self.validate(target).is_ok()
    }
}

impl<S: Signature, P: PublicKey<S>> TryFrom<IdCert<S, P>> for Certificate {
    type Error = ConversionError;
    fn try_from(value: IdCert<S, P>) -> Result<Self, Self::Error> {
        Ok(Self {
            tbs_certificate: value.id_cert_tbs.clone().try_into()?,
            signature_algorithm: value.id_cert_tbs.signature_algorithm,
            signature: value.signature.to_bitstring()?,
        })
    }
}

impl<S: Signature, P: PublicKey<S>> TryFrom<Certificate> for IdCert<S, P> {
    type Error = ConversionError;
    /// Tries to convert a [Certificate] into an [IdCert]. The Ok() variant of this method
    /// contains the `IdCert` if the conversion was successful. If this conversion is called
    /// manually, the caller is responsible for verifying the correctness of this `IdCert` using
    /// the [Constrained] trait.
    fn try_from(value: Certificate) -> Result<Self, Self::Error> {
        let id_cert_tbs = value.tbs_certificate.try_into()?;
        let signature = S::from_bytes(value.signature.raw_bytes());
        let cert = IdCert {
            id_cert_tbs,
            signature,
        };
        Ok(cert)
    }
}
