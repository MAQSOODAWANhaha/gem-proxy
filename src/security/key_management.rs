// src/security/key_management.rs
//! 密钥管理和保护
//! 
//! 提供密钥生成、轮换、存储和保护功能

use crate::error::{GeminiProxyError, ErrorSeverity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// 密钥类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyType {
    /// JWT签名密钥
    JwtSecret,
    /// API密钥
    ApiKey,
    /// 加密密钥
    EncryptionKey,
    /// 会话密钥
    SessionKey,
}

/// 密钥强度级别
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KeyStrength {
    /// 弱 (< 128 位)
    Weak,
    /// 中等 (128-191 位)
    Medium,
    /// 强 (192-255 位)
    Strong,
    /// 极强 (> 256 位)
    VeryStrong,
}

/// 密钥信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    /// 密钥ID
    pub key_id: String,
    /// 密钥类型
    pub key_type: KeyType,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 最后使用时间
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    /// 过期时间
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 密钥强度
    pub strength: KeyStrength,
    /// 使用次数
    pub usage_count: u64,
    /// 是否启用
    pub is_active: bool,
}

/// 密钥生成器
pub struct SecureKeyGenerator;

impl SecureKeyGenerator {
    /// 生成安全的JWT密钥
    pub fn generate_jwt_secret(length: usize) -> Result<String, GeminiProxyError> {
        if length < 32 {
            return Err(GeminiProxyError::config_with_context(
                "JWT密钥长度至少需要32字符",
                "key_generator",
                "generate_jwt_secret"
            ));
        }

        let charset = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;:,.<>?";
        let mut secret = String::with_capacity(length);

        for _ in 0..length {
            let idx = rand::random::<usize>() % charset.len();
            secret.push(charset.chars().nth(idx).unwrap());
        }

        Ok(secret)
    }

    /// 生成加密安全的随机字符串
    pub fn generate_secure_random(length: usize, include_special: bool) -> String {
        let charset = if include_special {
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-="
        } else {
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
        };

        (0..length)
            .map(|_| {
                let idx = rand::random::<usize>() % charset.len();
                charset.chars().nth(idx).unwrap()
            })
            .collect()
    }

    /// 生成十六进制编码的随机密钥
    pub fn generate_hex_key(bytes: usize) -> String {
        (0..bytes)
            .map(|_| format!("{:02x}", rand::random::<u8>()))
            .collect()
    }

    /// 生成Base64编码的随机密钥
    pub fn generate_base64_key(bytes: usize) -> String {
        use base64::{Engine as _, engine::general_purpose};
        let random_bytes: Vec<u8> = (0..bytes).map(|_| rand::random::<u8>()).collect();
        general_purpose::STANDARD.encode(&random_bytes)
    }
}

/// 密钥强度分析器
pub struct KeyStrengthAnalyzer;

impl KeyStrengthAnalyzer {
    /// 分析密钥强度
    pub fn analyze_strength(key: &str, key_type: &KeyType) -> KeyStrength {
        let entropy = Self::calculate_entropy(key);
        let length_score = Self::calculate_length_score(key, key_type);
        let complexity_score = Self::calculate_complexity_score(key);

        let total_score = (entropy + length_score + complexity_score) / 3.0;

        match total_score {
            s if s >= 90.0 => KeyStrength::VeryStrong,
            s if s >= 70.0 => KeyStrength::Strong,
            s if s >= 50.0 => KeyStrength::Medium,
            _ => KeyStrength::Weak,
        }
    }

    /// 计算密钥熵值
    fn calculate_entropy(key: &str) -> f64 {
        if key.is_empty() {
            return 0.0;
        }

        let mut char_counts = HashMap::new();
        for c in key.chars() {
            *char_counts.entry(c).or_insert(0) += 1;
        }

        let length = key.len() as f64;
        let entropy = char_counts
            .values()
            .map(|&count| {
                let frequency = count as f64 / length;
                -frequency * frequency.log2()
            })
            .sum::<f64>();

        // 标准化到 0-100 范围
        (entropy / 8.0 * 100.0).min(100.0)
    }

    /// 计算长度评分
    fn calculate_length_score(key: &str, key_type: &KeyType) -> f64 {
        let min_length = match key_type {
            KeyType::JwtSecret => 32,
            KeyType::ApiKey => 20,
            KeyType::EncryptionKey => 32,
            KeyType::SessionKey => 16,
        };

        let optimal_length = min_length * 2;
        let length = key.len();

        if length < min_length {
            0.0
        } else if length >= optimal_length {
            100.0
        } else {
            (length - min_length) as f64 / (optimal_length - min_length) as f64 * 100.0
        }
    }

    /// 计算复杂度评分
    fn calculate_complexity_score(key: &str) -> f64 {
        let has_lowercase = key.chars().any(|c| c.is_ascii_lowercase());
        let has_uppercase = key.chars().any(|c| c.is_ascii_uppercase());
        let has_digit = key.chars().any(|c| c.is_ascii_digit());
        let has_special = key.chars().any(|c| !c.is_ascii_alphanumeric());

        let features = [has_lowercase, has_uppercase, has_digit, has_special];
        let feature_count = features.iter().filter(|&&x| x).count();

        (feature_count as f64 / 4.0) * 100.0
    }

    /// 检查是否为常见弱密钥
    pub fn is_weak_key(key: &str) -> bool {
        let weak_keys = [
            "password", "123456", "admin", "secret", "changeme",
            "default", "guest", "test", "demo", "example",
            "your-secret-key", "jwt-secret", "api-key",
        ];

        let key_lower = key.to_lowercase();
        weak_keys.iter().any(|&weak| key_lower.contains(weak))
    }
}

/// 密钥轮换管理器
pub struct KeyRotationManager {
    /// 密钥信息存储
    key_info: HashMap<String, KeyInfo>,
    /// 轮换策略
    rotation_policies: HashMap<KeyType, RotationPolicy>,
}

/// 密钥轮换策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationPolicy {
    /// 轮换间隔（天）
    pub rotation_interval_days: u32,
    /// 是否启用自动轮换
    pub auto_rotation_enabled: bool,
    /// 轮换前的警告时间（天）
    pub warning_days: u32,
    /// 最大使用次数
    pub max_usage_count: Option<u64>,
}

impl Default for RotationPolicy {
    fn default() -> Self {
        Self {
            rotation_interval_days: 90, // 3个月
            auto_rotation_enabled: false,
            warning_days: 7,
            max_usage_count: None,
        }
    }
}

impl KeyRotationManager {
    /// 创建新的密钥轮换管理器
    pub fn new() -> Self {
        let mut rotation_policies = HashMap::new();
        
        // JWT密钥策略：6个月轮换
        rotation_policies.insert(KeyType::JwtSecret, RotationPolicy {
            rotation_interval_days: 180,
            auto_rotation_enabled: false,
            warning_days: 14,
            max_usage_count: None,
        });

        // API密钥策略：1年轮换
        rotation_policies.insert(KeyType::ApiKey, RotationPolicy {
            rotation_interval_days: 365,
            auto_rotation_enabled: false,
            warning_days: 30,
            max_usage_count: Some(1_000_000),
        });

        Self {
            key_info: HashMap::new(),
            rotation_policies,
        }
    }

    /// 注册密钥
    pub fn register_key(&mut self, key_id: String, key_type: KeyType, key_value: &str) {
        let strength = KeyStrengthAnalyzer::analyze_strength(key_value, &key_type);
        
        let key_info = KeyInfo {
            key_id: key_id.clone(),
            key_type,
            created_at: chrono::Utc::now(),
            last_used: None,
            expires_at: None,
            strength,
            usage_count: 0,
            is_active: true,
        };

        self.key_info.insert(key_id, key_info);
    }

    /// 记录密钥使用
    pub fn record_usage(&mut self, key_id: &str) {
        if let Some(info) = self.key_info.get_mut(key_id) {
            info.usage_count += 1;
            info.last_used = Some(chrono::Utc::now());
        }
    }

    /// 检查是否需要轮换
    pub fn needs_rotation(&self, key_id: &str) -> bool {
        if let Some(info) = self.key_info.get(key_id) {
            if let Some(policy) = self.rotation_policies.get(&info.key_type) {
                let now = chrono::Utc::now();
                let age = now - info.created_at;
                
                // 检查时间是否超过轮换间隔
                if age.num_days() >= policy.rotation_interval_days as i64 {
                    return true;
                }

                // 检查使用次数是否超过限制
                if let Some(max_usage) = policy.max_usage_count {
                    if info.usage_count >= max_usage {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// 获取轮换警告
    pub fn get_rotation_warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let now = chrono::Utc::now();

        for (key_id, info) in &self.key_info {
            if let Some(policy) = self.rotation_policies.get(&info.key_type) {
                let age = now - info.created_at;
                let warning_threshold = policy.rotation_interval_days - policy.warning_days;

                if age.num_days() >= warning_threshold as i64 {
                    warnings.push(format!(
                        "密钥 '{}' ({:?}) 将在 {} 天后需要轮换",
                        key_id,
                        info.key_type,
                        policy.rotation_interval_days as i64 - age.num_days()
                    ));
                }
            }
        }

        warnings
    }

    /// 获取密钥统计信息
    pub fn get_key_statistics(&self) -> KeyStatistics {
        let mut stats = KeyStatistics::default();
        
        for info in self.key_info.values() {
            stats.total_keys += 1;
            
            if info.is_active {
                stats.active_keys += 1;
            }

            match info.strength {
                KeyStrength::Weak => stats.weak_keys += 1,
                KeyStrength::Medium => stats.medium_keys += 1,
                KeyStrength::Strong => stats.strong_keys += 1,
                KeyStrength::VeryStrong => stats.very_strong_keys += 1,
            }

            if self.needs_rotation(&info.key_id) {
                stats.keys_needing_rotation += 1;
            }
        }

        stats
    }
}

/// 密钥统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyStatistics {
    pub total_keys: usize,
    pub active_keys: usize,
    pub weak_keys: usize,
    pub medium_keys: usize,
    pub strong_keys: usize,
    pub very_strong_keys: usize,
    pub keys_needing_rotation: usize,
}

/// 安全密钥存储
pub struct SecureKeyStorage;

impl SecureKeyStorage {
    /// 安全存储密钥到文件
    pub fn store_key_securely(
        key_value: &str,
        file_path: &Path,
        permissions: Option<u32>,
    ) -> Result<(), GeminiProxyError> {
        use std::fs::OpenOptions;
        use std::io::Write;

        // 创建文件
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(file_path)
            .map_err(|e| GeminiProxyError::storage(format!("无法创建密钥文件: {}", e)))?;

        // 写入密钥
        file.write_all(key_value.as_bytes())
            .map_err(|e| GeminiProxyError::storage(format!("无法写入密钥: {}", e)))?;

        // 设置文件权限（仅限Unix系统）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(permissions.unwrap_or(0o600));
            std::fs::set_permissions(file_path, perms)
                .map_err(|e| GeminiProxyError::storage(format!("无法设置文件权限: {}", e)))?;
        }

        Ok(())
    }

    /// 从文件安全读取密钥
    pub fn load_key_securely(file_path: &Path) -> Result<String, GeminiProxyError> {
        // 检查文件权限
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = std::fs::metadata(file_path)
                .map_err(|e| GeminiProxyError::storage(format!("无法读取文件元数据: {}", e)))?;
            
            let mode = metadata.mode() & 0o777;
            if mode & 0o077 != 0 {
                return Err(GeminiProxyError::config_with_context(
                    "密钥文件权限过于宽松，存在安全风险",
                    "key_storage",
                    "load_key"
                ).with_severity(ErrorSeverity::Critical));
            }
        }

        std::fs::read_to_string(file_path)
            .map_err(|e| GeminiProxyError::storage(format!("无法读取密钥文件: {}", e)))
    }

    /// 生成密钥备份
    pub fn backup_key(
        key_value: &str,
        backup_path: &Path,
        encryption_key: Option<&str>,
    ) -> Result<(), GeminiProxyError> {
        let content = if let Some(enc_key) = encryption_key {
            // 简单的XOR加密（实际应用中应使用更强的加密）
            Self::simple_encrypt(key_value, enc_key)
        } else {
            key_value.to_string()
        };

        Self::store_key_securely(&content, backup_path, Some(0o600))
    }

    /// 简单的XOR加密（仅用于演示）
    fn simple_encrypt(data: &str, key: &str) -> String {
        let key_bytes = key.as_bytes();
        data.bytes()
            .enumerate()
            .map(|(i, b)| b ^ key_bytes[i % key_bytes.len()])
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_key_generation() {
        let jwt_secret = SecureKeyGenerator::generate_jwt_secret(64).unwrap();
        assert_eq!(jwt_secret.len(), 64);
        
        let hex_key = SecureKeyGenerator::generate_hex_key(32);
        assert_eq!(hex_key.len(), 64); // 32 bytes = 64 hex chars
        
        let random_string = SecureKeyGenerator::generate_secure_random(20, true);
        assert_eq!(random_string.len(), 20);
    }

    #[test]
    fn test_key_strength_analysis() {
        // 弱密钥
        let weak_key = "password";
        let strength = KeyStrengthAnalyzer::analyze_strength(weak_key, &KeyType::JwtSecret);
        assert_eq!(strength, KeyStrength::Weak);
        
        // 强密钥
        let strong_key = "VeryStr0ng!P@ssw0rd#With$pecial%Ch@rs&";
        let strength = KeyStrengthAnalyzer::analyze_strength(strong_key, &KeyType::JwtSecret);
        assert!(matches!(strength, KeyStrength::Strong | KeyStrength::VeryStrong));
    }

    #[test]
    fn test_weak_key_detection() {
        assert!(KeyStrengthAnalyzer::is_weak_key("password"));
        assert!(KeyStrengthAnalyzer::is_weak_key("your-secret-key"));
        assert!(!KeyStrengthAnalyzer::is_weak_key("VeryStr0ng!RandomKey#123"));
    }

    #[test]
    fn test_key_rotation_manager() {
        let mut manager = KeyRotationManager::new();
        
        manager.register_key(
            "test-key".to_string(),
            KeyType::JwtSecret,
            "strong-test-key-123456789"
        );
        
        // 新密钥不应该需要轮换
        assert!(!manager.needs_rotation("test-key"));
        
        // 记录使用
        manager.record_usage("test-key");
        
        let stats = manager.get_key_statistics();
        assert_eq!(stats.total_keys, 1);
        assert_eq!(stats.active_keys, 1);
    }

    #[test]
    fn test_secure_key_storage() {
        let temp_file = NamedTempFile::new().unwrap();
        let test_key = "test-secret-key-12345";
        
        // 存储密钥
        SecureKeyStorage::store_key_securely(
            test_key,
            temp_file.path(),
            Some(0o600)
        ).unwrap();
        
        // 读取密钥
        let loaded_key = SecureKeyStorage::load_key_securely(temp_file.path()).unwrap();
        assert_eq!(loaded_key, test_key);
    }
}