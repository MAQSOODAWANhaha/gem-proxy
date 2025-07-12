// src/utils/tls.rs
use crate::config::AcmeConfig;
use crate::proxy::acme_service::AcmeChallengeState;
use acme_lib::persist::FilePersist;
use acme_lib::{Directory, DirectoryUrl};
use openssl::x509::X509;
use rcgen::generate_simple_self_signed;
use std::fs;
use std::path::Path;
use std::time::Duration;

pub fn generate_self_signed_cert_if_not_exists(
    cert_path: &str,
    key_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if Path::new(cert_path).exists() && Path::new(key_path).exists() {
        tracing::info!("Certificate and key already exist at {} and {}", cert_path, key_path);
        return Ok(());
    }

    tracing::info!("Generating self-signed certificate for HTTPS...");
    
    // 生成更全面的自签证书，包含多个 SAN
    let subject_alt_names = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),                    // IPv6 localhost
        "gemini-proxy".to_string(),           // 容器名
        "gemini-proxy.local".to_string(),     // 本地域名
        "proxy.local".to_string(),            // 简短别名
    ];

    let cert = generate_simple_self_signed(subject_alt_names)?;

    // rcgen 0.13.1: use cert.serialize_pem() and cert.key_pair.serialize_pem()
    let cert_pem = cert.cert.pem();
    let key_pem = cert.key_pair.serialize_pem();

    // 确保目录存在
    if let Some(parent) = Path::new(cert_path).parent() {
        fs::create_dir_all(parent)?;
        tracing::debug!("Created certificate directory: {:?}", parent);
    }
    if let Some(parent) = Path::new(key_path).parent() {
        fs::create_dir_all(parent)?;
        tracing::debug!("Created key directory: {:?}", parent);
    }

    // 写入证书和私钥
    fs::write(cert_path, cert_pem)?;
    fs::write(key_path, key_pem)?;

    // 设置适当的文件权限（私钥只读）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(key_path)?.permissions();
        perms.set_mode(0o600); // 只有所有者可读写
        fs::set_permissions(key_path, perms)?;
        tracing::debug!("Set restrictive permissions on private key");
    }

    tracing::info!("✅ Self-signed certificate generated successfully!");
    tracing::info!("📄 Certificate: {}", cert_path);
    tracing::info!("🔑 Private key: {}", key_path);
    tracing::info!("🌐 Valid for: localhost, 127.0.0.1, ::1, gemini-proxy, *.local");

    Ok(())
}

pub async fn manage_acme_certificate(
    config: &AcmeConfig,
    challenge_state: AcmeChallengeState,
    cert_path: &str,
    key_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!(
        "Starting ACME certificate management for domains: {:?}",
        &config.domains
    );

    let url = DirectoryUrl::Other(&config.directory_url);
    let persist = FilePersist::new(".");
    let dir = Directory::from_url(persist, url)?;
    let acc = dir.account(&config.email)?;

    let domains_str: Vec<&str> = config.domains.iter().map(|s| s.as_str()).collect();
    let mut ord = acc.new_order(&domains_str[0], &domains_str)?;

    let ord_csr = loop {
        if let Some(ord_csr) = ord.confirm_validations() {
            break ord_csr;
        }

        let auths = ord.authorizations()?;
        let chall = auths[0].http_challenge();

        let token = chall.http_token();
        // acme-lib 0.5.2: The key authorization is the token for HTTP challenge
        let key_auth = token.to_string();

        {
            let mut state = challenge_state.write().unwrap();
            state.insert(token.to_string(), key_auth.to_string());
            tracing::info!("ACME challenge token set for domain validation.");
        }

        chall.validate(5000)?;
        ord.refresh()?;
        tokio::time::sleep(Duration::from_secs(2)).await;
    };

    let pkey = acme_lib::create_p384_key();
    // acme-lib 0.5.2: finalize_pkey needs private key, public key, and timeout
    let ord_cert = ord_csr.finalize_pkey(pkey.0, pkey.1, 5000)?;
    let cert = ord_cert.download_and_save_cert()?;

    fs::write(cert_path, cert.certificate())?;
    fs::write(key_path, cert.private_key())?;

    tracing::info!("ACME certificate and private key have been successfully obtained and saved.");
    Ok(())
}

const RENEW_BEFORE_DAYS: i32 = 30;

fn needs_renewal(cert_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    if !Path::new(cert_path).exists() {
        tracing::info!("Certificate does not exist, renewal needed.");
        return Ok(true);
    }

    let cert_bytes = fs::read(cert_path)?;
    let cert = X509::from_pem(&cert_bytes)?;

    let not_after = cert.not_after();
    let now = openssl::asn1::Asn1Time::days_from_now(0)?;

    let diff = now.diff(not_after)?;

    if diff.days < RENEW_BEFORE_DAYS {
        tracing::info!("Certificate expires in {} days, renewal needed.", diff.days);
        Ok(true)
    } else {
        tracing::info!(
            "Certificate is valid for {} more days. No renewal needed.",
            diff.days
        );
        Ok(false)
    }
}

pub async fn acme_renewal_loop(
    config: &AcmeConfig,
    challenge_state: AcmeChallengeState,
    cert_path: &str,
    key_path: &str,
) {
    loop {
        tracing::info!("Checking certificate renewal status...");
        match needs_renewal(cert_path) {
            Ok(true) => {
                tracing::info!("Proceeding with ACME certificate issuance/renewal.");
                if let Err(e) =
                    manage_acme_certificate(config, challenge_state.clone(), cert_path, key_path)
                        .await
                {
                    tracing::error!(
                        "ACME certificate management failed: {}. Retrying in 1 hour.",
                        e
                    );
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                    continue;
                }
            }
            Ok(false) => {
                tracing::info!("Certificate is up to date.");
            }
            Err(e) => {
                tracing::error!(
                    "Failed to check certificate status: {}. Retrying in 1 hour.",
                    e
                );
                tokio::time::sleep(Duration::from_secs(3600)).await;
                continue;
            }
        }

        tracing::info!("Next certificate renewal check in 24 hours.");
        tokio::time::sleep(Duration::from_secs(24 * 3600)).await;
    }
}
