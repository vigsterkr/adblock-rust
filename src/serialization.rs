use serde;
use serde::{Serialize, Deserialize};

use zstd::stream::write::Encoder;
use zstd::stream::read::Decoder;
use zstd;

use bincode;

use crate::blocker::Blocker;

// Pick version to use for serialization from cargo package version
pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");

// Helper structs to use the wrapped Blocker struct with its own serialization definitions
pub struct Wrapper<'a> {
    pub wrapped: &'a Blocker
}

pub struct Unwrappable {
    pub wrapped: Box<Blocker>
}

impl<'a> serde::Serialize for Wrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // An intermediate struct that adds manifest version and includes
        // the wrapped structure already encoded. Allows for checking of
        // `manifest-version` before any other field gets decoded
        #[derive(Serialize)]
        struct EncodedBlocker<'b> {
            #[serde(rename = "manifest-version")]
            manifest_version: &'b str,
            blocker: &'b Vec<u8>
        }

        let mut zst = Encoder::new(Vec::new(), zstd::DEFAULT_COMPRESSION_LEVEL).unwrap();

        bincode::serialize_into(&mut zst, &self)
            .or_else(|e| {
                Err(D::Error::invalid_value(::serde::de::Unexpected::Other("Failed to serialize to bincode"), &e.to_string().as_str()))
            })?;

        let compressed = zst.finish().unwrap();
            .or_else(|e| {
                Err(D::Error::invalid_value(::serde::de::Unexpected::Other("Failed to finish Gzip encoding"), &e.to_string().as_str()))
            })?;

        let output = EncodedBlocker {
            // Pick version to use for serialization from cargo package version
            manifest_version: VERSION,
            blocker: &compressed,
        };

        // Once again, serde does all the hard work for us
        output.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Unwrappable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        // An intermediate struct that exactly matches the input schema.
        #[derive(Deserialize)]
        struct EncodedBlocker {
            #[serde(rename = "manifest-version")]
            pub manifest_version: String,
            pub blocker: Vec<u8>
        }

        // Because we derived Deserialize automatically,
        // serde does all the hard work for us.
        let input = EncodedBlocker::deserialize(deserializer)?;

        // Validating the manifest_version field is straightforward.
        if input.manifest_version != VERSION {
            return Err(D::Error::invalid_value(
                ::serde::de::Unexpected::Str(&input.manifest_version), &VERSION
            ));
        }

        let zst = Decoder::new(serialized).unwrap();
        let blocker = bincode::deserialize_from(zst)
            .or_else(|e| {
                Err(D::Error::invalid_value(::serde::de::Unexpected::Other("Failed to parse bincode formatted data"), &e.to_string().as_str()))
            })?;

        // Finally, we move all the data into an instance
        // of our wrapper struct.
        Ok(Unwrappable {
            wrapped: Box::new(blocker)
        })
    }
}

