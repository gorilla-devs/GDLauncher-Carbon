//! Base URLs for the Microsoft, Xbox, and Minecraft services that account
//! enrollment talks to, and the key its entitlement response is verified
//! against.
//!
//! Every value defaults to the real host. Under the `e2e` feature only,
//! `--e2e_auth_base <url>` redirects all four services at a local mock and
//! `--e2e_entitlement_key <path>` supplies the public key that replaces
//! Mojang's, so the Playwright suite can drive a complete enrollment with no
//! network.
//!
//! The feature is absent from every published build. The device-code flow
//! shows the user a `verification_uri` that the identity provider chooses, so
//! a redirectable provider in a shipped binary is a working phishing page.
//! Gating on `not(feature = "production")` would not be enough: snapshot
//! builds reach real users and are compiled without that feature.

// Belt-and-braces alongside the "never built into a shipped artifact"
// contract above: this makes the dangerous combination fail to compile
// rather than rely solely on build scripts never requesting it.
#[cfg(all(feature = "e2e", feature = "production"))]
compile_error!("the e2e overrides must never be compiled into a production build");

use std::sync::OnceLock;

pub const MS_LOGIN_DEFAULT: &str = "https://login.microsoftonline.com";
pub const XBL_DEFAULT: &str = "https://user.auth.xboxlive.com";
pub const XSTS_DEFAULT: &str = "https://xsts.auth.xboxlive.com";
pub const MC_SERVICES_DEFAULT: &str = "https://api.minecraftservices.com";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoints {
    pub ms_login: String,
    pub xbl: String,
    pub xsts: String,
    pub mc_services: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            ms_login: MS_LOGIN_DEFAULT.to_string(),
            xbl: XBL_DEFAULT.to_string(),
            xsts: XSTS_DEFAULT.to_string(),
            mc_services: MC_SERVICES_DEFAULT.to_string(),
        }
    }
}

impl Endpoints {
    /// All four services pointed at a single mock, separated by path prefix.
    ///
    /// One base means one flag, one port, and routes that name the service
    /// they stand in for.
    pub fn rooted_at(base: &str) -> Self {
        let base = base.trim_end_matches('/');

        Self {
            ms_login: format!("{base}/ms"),
            xbl: format!("{base}/xbl"),
            xsts: format!("{base}/xsts"),
            mc_services: format!("{base}/mc"),
        }
    }
}

static ENDPOINTS: OnceLock<Endpoints> = OnceLock::new();

fn endpoints() -> &'static Endpoints {
    ENDPOINTS.get_or_init(Endpoints::default)
}

/// PEM of the public key that verifies the entitlement JWT, when overridden.
///
/// `None` means Mojang's bundled key is used. Only ever `Some` under the `e2e`
/// feature with `--e2e_entitlement_key` supplied.
#[cfg(feature = "e2e")]
static ENTITLEMENT_KEY: OnceLock<Vec<u8>> = OnceLock::new();

#[cfg(feature = "e2e")]
pub fn e2e_entitlement_key() -> Option<&'static [u8]> {
    ENTITLEMENT_KEY.get().map(Vec::as_slice)
}

/// The value following `name` in `args`, or `None` when the flag is absent or
/// is the final token.
pub(crate) fn arg_value<I: Iterator<Item = String>>(mut args: I, name: &str) -> Option<String> {
    while let Some(arg) = args.next() {
        if arg == name {
            return args.next();
        }
    }

    None
}

/// The endpoints the e2e arguments ask for, or `None` when no override is present.
///
/// Split from `init_from_args` so the argument contract can be asserted without
/// touching the process-wide `ENDPOINTS`.
#[cfg(feature = "e2e")]
pub(crate) fn endpoints_from_args<I: Iterator<Item = String>>(args: I) -> Option<Endpoints> {
    arg_value(args, "--e2e_auth_base").map(|base| Endpoints::rooted_at(&base))
}

/// Applies the e2e overrides from the process arguments.
///
/// Call once at startup, before anything reads an endpoint. Without the `e2e`
/// feature this is an empty function and the arguments are ignored.
pub fn init_from_args() {
    #[cfg(feature = "e2e")]
    {
        if let Some(endpoints) = endpoints_from_args(std::env::args()) {
            tracing::warn!("E2E MODE: auth endpoints redirected to {endpoints:?}");

            ENDPOINTS
                .set(endpoints)
                .expect("auth endpoints were read before init_from_args ran");
        }

        if let Some(path) = arg_value(std::env::args(), "--e2e_entitlement_key") {
            tracing::warn!("E2E MODE: entitlement key read from {path}");

            // A missing or unreadable key would otherwise surface much later
            // as an opaque signature failure mid-enrollment.
            let pem = std::fs::read(&path)
                .unwrap_or_else(|e| panic!("cannot read the e2e entitlement key at {path}: {e}"));

            // Parsed once here and the result discarded: a key that reads
            // fine but isn't a valid RSA public key PEM must fail at startup,
            // not resurface as an opaque signature failure mid-enrollment.
            jsonwebtoken::DecodingKey::from_rsa_pem(&pem).unwrap_or_else(|e| {
                panic!("the e2e entitlement key at {path} is not a valid RSA public key PEM: {e}")
            });

            ENTITLEMENT_KEY.set(pem).expect("init_from_args ran twice");
        }
    }
}

pub fn ms_login() -> &'static str {
    &endpoints().ms_login
}

pub fn xbl() -> &'static str {
    &endpoints().xbl
}

pub fn xsts() -> &'static str {
    &endpoints().xsts
}

pub fn mc_services() -> &'static str {
    &endpoints().mc_services
}

pub fn device_code_url() -> String {
    format!("{}/consumers/oauth2/v2.0/devicecode", ms_login())
}

pub fn ms_token_url() -> String {
    format!("{}/consumers/oauth2/v2.0/token", ms_login())
}

pub fn ms_authorize_url() -> String {
    format!("{}/consumers/oauth2/v2.0/authorize", ms_login())
}

pub fn xbl_authenticate_url() -> String {
    format!("{}/user/authenticate", xbl())
}

pub fn xsts_authorize_url() -> String {
    format!("{}/xsts/authorize", xsts())
}

pub fn mc_login_with_xbox_url() -> String {
    format!("{}/authentication/login_with_xbox", mc_services())
}

pub fn mc_entitlements_url() -> String {
    format!("{}/entitlements/mcstore", mc_services())
}

pub fn mc_profile_url() -> String {
    format!("{}/minecraft/profile", mc_services())
}

pub fn mc_name_availability_url(username: &str) -> String {
    format!(
        "{}/minecraft/profile/name/{}/available",
        mc_services(),
        username
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn defaults_are_the_real_hosts() {
        let e = Endpoints::default();

        assert_eq!(e.ms_login, "https://login.microsoftonline.com");
        assert_eq!(e.xbl, "https://user.auth.xboxlive.com");
        assert_eq!(e.xsts, "https://xsts.auth.xboxlive.com");
        assert_eq!(e.mc_services, "https://api.minecraftservices.com");
    }

    #[test]
    fn rooted_at_splits_one_base_by_path_prefix() {
        let e = Endpoints::rooted_at("http://127.0.0.1:9999");

        assert_eq!(e.ms_login, "http://127.0.0.1:9999/ms");
        assert_eq!(e.xbl, "http://127.0.0.1:9999/xbl");
        assert_eq!(e.xsts, "http://127.0.0.1:9999/xsts");
        assert_eq!(e.mc_services, "http://127.0.0.1:9999/mc");
    }

    #[test]
    fn rooted_at_tolerates_a_trailing_slash() {
        // A URL pasted from a browser or built by string concat often carries
        // one; doubling it would 404 against the mock's exact-match routes.
        let e = Endpoints::rooted_at("http://127.0.0.1:9999/");

        assert_eq!(e.ms_login, "http://127.0.0.1:9999/ms");
    }

    #[test]
    fn arg_value_reads_the_token_after_the_flag() {
        let args = ["prog", "--other", "x", "--e2e_auth_base", "http://host"]
            .into_iter()
            .map(String::from);

        assert_eq!(
            arg_value(args, "--e2e_auth_base"),
            Some("http://host".to_string())
        );
    }

    #[test]
    fn arg_value_is_none_when_the_flag_is_absent() {
        let args = ["prog", "--other", "x"].into_iter().map(String::from);

        assert_eq!(arg_value(args, "--e2e_auth_base"), None);
    }

    #[test]
    fn arg_value_is_none_when_the_flag_is_last() {
        // Guards against consuming the next flag as if it were a value.
        let args = ["prog", "--e2e_auth_base"].into_iter().map(String::from);

        assert_eq!(arg_value(args, "--e2e_auth_base"), None);
    }

    #[test]
    #[cfg(feature = "e2e")]
    fn endpoints_from_args_is_none_when_the_flag_is_absent() {
        let args = ["prog", "--other", "x"].into_iter().map(String::from);

        assert_eq!(endpoints_from_args(args), None);
    }

    #[test]
    #[cfg(feature = "e2e")]
    fn endpoints_from_args_roots_all_four_urls_at_the_base_when_the_flag_is_present() {
        let args = ["prog", "--e2e_auth_base", "http://127.0.0.1:9999"]
            .into_iter()
            .map(String::from);

        assert_eq!(
            endpoints_from_args(args),
            Some(Endpoints::rooted_at("http://127.0.0.1:9999"))
        );
    }

    #[test]
    #[cfg(feature = "e2e")]
    fn endpoints_from_args_names_the_flag_e2e_auth_base() {
        // `--e2e_auth_base` is a cross-language contract: the Electron side
        // forwards this exact literal on the command line. A typo on either
        // side must fail loudly here, not in a Playwright run that silently
        // talks to real Microsoft and Xbox endpoints.
        let args = ["prog", "--e2e_auth_base", "http://host"]
            .into_iter()
            .map(String::from);

        assert_eq!(
            endpoints_from_args(args),
            Some(Endpoints::rooted_at("http://host"))
        );

        let differently_spelled_flag = ["prog", "--e2e-auth-base", "http://host"]
            .into_iter()
            .map(String::from);

        assert_eq!(endpoints_from_args(differently_spelled_flag), None);
    }

    #[test]
    fn urls_default_to_the_documented_endpoints() {
        // These must match the literals `api.rs` and `enroll.rs` use verbatim. If one
        // drifts, enrollment silently talks to the wrong host in production.
        assert_eq!(
            device_code_url(),
            "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode"
        );
        assert_eq!(
            ms_token_url(),
            "https://login.microsoftonline.com/consumers/oauth2/v2.0/token"
        );
        assert_eq!(
            ms_authorize_url(),
            "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize"
        );
        assert_eq!(
            xbl_authenticate_url(),
            "https://user.auth.xboxlive.com/user/authenticate"
        );
        assert_eq!(
            xsts_authorize_url(),
            "https://xsts.auth.xboxlive.com/xsts/authorize"
        );
        assert_eq!(
            mc_login_with_xbox_url(),
            "https://api.minecraftservices.com/authentication/login_with_xbox"
        );
        assert_eq!(
            mc_entitlements_url(),
            "https://api.minecraftservices.com/entitlements/mcstore"
        );
        assert_eq!(
            mc_profile_url(),
            "https://api.minecraftservices.com/minecraft/profile"
        );
        assert_eq!(
            mc_name_availability_url("Notch"),
            "https://api.minecraftservices.com/minecraft/profile/name/Notch/available"
        );
    }

    #[cfg(feature = "e2e")]
    #[test]
    fn entitlement_key_is_unset_without_the_flag() {
        // `cargo test` argv never carries --e2e_entitlement_key, so the
        // override must stay dormant and Mojang's bundled key must win.
        assert!(e2e_entitlement_key().is_none());
    }
}
