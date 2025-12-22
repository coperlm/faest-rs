use faest::aes::Aes;
use faest::fiat_shamir::{SignatureVerifier, Signer};
use faest::gf2psmall::GF2p8;
use faest::{keygen, FaestSignatureVerifier, FaestSigner};

fn main() {
    println!("FAEST 签名方案演示\n");
    println!("{}", "=".repeat(50));

    // 生成密钥对
    println!("\n1. 生成密钥对...");
    let (secret_key, public_key) = keygen(Aes::Aes128);
    println!("   ✓ 密钥对生成成功");

    // 要签名的消息
    let message = b"Hello, FAEST! This is a post-quantum signature scheme.";
    println!("\n2. 消息: {}", String::from_utf8_lossy(message));

    // 创建签名
    println!("\n3. 创建签名...");
    let signer = FaestSigner::<GF2p8>::new(secret_key, public_key);
    let signature = signer.sign(message);
    println!("   ✓ 签名创建成功");
    println!("   签名大小: {} 字节", signature.to_bytes().len());

    // 验证签名
    println!("\n4. 验证签名...");
    let verifier = FaestSignatureVerifier::<GF2p8>::new(Aes::Aes128, public_key);
    let is_valid = verifier.verify(&signature, message);
    
    if is_valid {
        println!("   ✓ 签名验证成功！");
    } else {
        println!("   ✗ 签名验证失败！");
    }

    // 测试错误的消息
    println!("\n5. 测试篡改的消息...");
    let wrong_message = b"Hello, FAEST! This is a MODIFIED message.";
    let verifier2 = FaestSignatureVerifier::<GF2p8>::new(Aes::Aes128, public_key);
    let is_valid2 = verifier2.verify(&signature, wrong_message);
    
    if !is_valid2 {
        println!("   ✓ 正确拒绝了篡改的消息");
    } else {
        println!("   ✗ 错误地接受了篡改的消息");
    }

    println!("\n{}", "=".repeat(50));
    println!("演示完成！");
}
