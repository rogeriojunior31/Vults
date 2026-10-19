use super::super::speaker_models::{EMBEDDING_SHA256, SEGMENTATION_SHA256};

pub const ID: &str = "meeting-analysis-linux-x64-20261018";
pub const RUNTIME_SHA256: &str = "469849977cb60744ea160927e1a3a0643fd4c55c0aff743251c6d293402e334e";

#[derive(Debug, Clone, Copy)]
pub struct FileSpec {
    pub name: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum Format {
    Raw,
    Gzip,
    Bzip2,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct Download {
    pub id: &'static str,
    pub url: &'static str,
    pub size: u64,
    pub sha256: &'static str,
    pub format: Format,
    pub members: &'static [(&'static str, FileSpec)],
}

pub const FILES: &[FileSpec] = &[
    FileSpec {
        name: "segmentation.onnx",
        size: 5_992_913,
        sha256: SEGMENTATION_SHA256,
    },
    FileSpec {
        name: "SEGMENTATION-LICENSE.txt",
        size: 1061,
        sha256: "14d7016ad68e7394d6e6b78d96cc2ae431c905287b89674cfdf021e79e62b8ba",
    },
    FileSpec {
        name: "embedding.onnx",
        size: 28_281_138,
        sha256: EMBEDDING_SHA256,
    },
    FileSpec {
        name: "libonnxruntime.so.1.31.0",
        size: 29_345_384,
        sha256: RUNTIME_SHA256,
    },
    FileSpec {
        name: "ONNXRUNTIME-LICENSE.txt",
        size: 1073,
        sha256: "2f07c72751aed99790b8a4869cf2311df85a860b22ded05fa22803587a48922c",
    },
    FileSpec {
        name: "ONNXRUNTIME-THIRD-PARTY-NOTICES.txt",
        size: 338088,
        sha256: "143764b952fdb1a7c69ce653bfba74a7744d6a8a573bfb73e235fba356c83de3",
    },
    FileSpec {
        name: "ONNXRUNTIME-PRIVACY.md",
        size: 6571,
        sha256: "ea592b16d154ff2de759ba5210350c64593c378c1bbd0a2aa4ddef777edbf358",
    },
];
pub(crate) const DOWNLOADS: &[Download] = &[
    Download {
        id: "segmentation",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-segmentation-models/sherpa-onnx-pyannote-segmentation-3-0.tar.bz2",
        size: 6_958_444,
        sha256: "24615ee884c897d9d2ba09bb4d30da6bb1b15e685065962db5b02e76e4996488",
        format: Format::Bzip2,
        members: &[
            ("sherpa-onnx-pyannote-segmentation-3-0/model.onnx", FILES[0]),
            ("sherpa-onnx-pyannote-segmentation-3-0/LICENSE", FILES[1]),
        ],
    },
    Download {
        id: "embedding",
        url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/speaker-recongition-models/3dspeaker_speech_campplus_sv_zh-cn_16k-common.onnx",
        size: 28_281_138,
        sha256: EMBEDDING_SHA256,
        format: Format::Raw,
        members: &[("", FILES[2])],
    },
    Download {
        id: "runtime",
        url: "https://github.com/microsoft/onnxruntime/releases/download/v1.31.0/onnxruntime-linux-x64-1.31.0.tgz",
        size: 11_408_003,
        sha256: "cc5c72baf5ae5c8238a6841f0897227be2d02826b9cf98eaf02fdefaeeb03a57",
        format: Format::Gzip,
        members: &[
            (
                "onnxruntime-linux-x64-1.31.0/lib/libonnxruntime.so.1.31.0",
                FILES[3],
            ),
            ("onnxruntime-linux-x64-1.31.0/LICENSE", FILES[4]),
            ("onnxruntime-linux-x64-1.31.0/ThirdPartyNotices.txt", FILES[5]),
            ("onnxruntime-linux-x64-1.31.0/Privacy.md", FILES[6]),
        ],
    },
];
pub(crate) const NOTICES: &[(&str, &[u8])] = &[
    (
        "BZIP2-LICENSE.txt",
        include_bytes!("../../../assets/meeting/BZIP2-LICENSE.txt"),
    ),
    (
        "CAMPLUS-LICENSE.txt",
        include_bytes!("../../../assets/meeting/CAMPLUS-LICENSE.txt"),
    ),
    (
        "CAMPLUS-NOTICE.txt",
        include_bytes!("../../../assets/meeting/CAMPLUS-NOTICE.txt"),
    ),
];
