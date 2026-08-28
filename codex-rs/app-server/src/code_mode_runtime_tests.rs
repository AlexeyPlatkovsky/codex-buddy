#![cfg(not(feature = "code-mode"))]

use std::io::ErrorKind;

use pretty_assertions::assert_eq;
use url::Url;

use super::preflight_transport;
use crate::CodeModeHostTransport;

#[test]
fn explicit_grpc_host_is_unavailable_before_startup() {
    let transport = CodeModeHostTransport::Grpc(
        Url::parse("https://example.test").expect("test endpoint should parse"),
    );

    let error = preflight_transport(&transport).expect_err("Slim must reject a code-mode host");

    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert_eq!(error.to_string(), "code mode is unavailable in this build");
}
