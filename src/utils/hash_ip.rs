use sha2::{Digest, Sha256};

/// Hash an IP address for privacy and storage
pub fn hash_ip(ip: &str) -> String {
    // Add a salt to prevent rainbow table attacks
    let salt = "makemeshort_salt"; // You should use an env var for this in production
    let salted_ip = format!("{}{}", ip, salt);

    let mut hasher = Sha256::new();
    hasher.update(salted_ip.as_bytes());
    let result = hasher.finalize();

    format!("{:x}", result)
}
