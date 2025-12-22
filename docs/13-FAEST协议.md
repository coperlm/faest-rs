# FAEST 协议核心

## 🎯 协议目标

证明：我知道 AES 密钥 `k`，使得 `AES(k, input) = output`

## 👥 两个角色

```rust
pub trait Prover {
    fn new(secret_key: SecretKey, public_key: PublicKey) -> Self;
    fn commit(&mut self) -> Commitment;
    fn commit_prove_consistency(&mut self, challenge: Challenge1) -> Response;
    fn prove(&mut self, challenge: Challenge2) -> Proof;
    fn transfer(&mut self, choice: Choice) -> Decommitment;
}

pub trait Verifier {
    fn new(aes: Aes, public_key: PublicKey) -> Self;
    fn receive_commitment(&mut self, commitment: Commitment);
    fn generate_challenge1(&mut self) -> Challenge1;
    fn receive_response(&mut self, response: Response);
    fn generate_challenge2(&mut self) -> Challenge2;
    fn verify(&mut self, proof: Proof, decom: Decommitment) -> bool;
}
```

## 🔑 密钥结构

```rust
pub enum SecretKey {
    Aes128Key { key: [u8; 16] },
    Aes192Key { key: [u8; 24] },
    Aes256Key { key: [u8; 32] },
}

pub struct PublicKey {
    pub input: [u8; 16],
    pub output: [u8; 16],
}
```

## 🎲 密钥生成

```rust
pub fn keygen(variant: Aes) -> (SecretKey, PublicKey) {
    loop {
        // 1. 随机生成密钥
        let key = random_bytes(variant.get_key_size());
        
        // 2. 检查密钥扩展（所有 S-Box 输入非零）
        let (round_keys, inv_outs) = expand_key_check(variant, &key);
        if has_zero_sbox_input(&inv_outs) {
            continue;  // 重新生成
        }
        
        // 3. 随机输入并加密
        let input = random_bytes(16);
        let (output, inv_outs) = encrypt_check(variant, input, &round_keys);
        if has_zero_sbox_input(&inv_outs) {
            continue;
        }
        
        // 4. 返回密钥对
        return (SecretKey::from(key), PublicKey { input, output });
    }
}
```

## 🔐 证明者类型

```rust
pub struct FaestProverFromHC<HCS: HomComSender> {
    secret_key: SecretKey,
    public_key: PublicKey,
    hc_sender: HCS,
    state: ProverState,
}
```

## 🔍 验证者类型

```rust
pub struct FaestVerifierFromHC<HCR: HomComReceiver> {
    aes: Aes,
    public_key: PublicKey,
    hc_receiver: HCR,
    state: VerifierState,
}
```

## 📐 见证（Witness）

扩展见证包含所有中间值：

```
[密钥] + [密钥扩展的 S-Box 输入] + [加密的 S-Box 输入]
```

```rust
let witness = compute_extended_witness(secret_key, input);
// witness.len() = key_size + 4*num_sbox_key + 16*num_rounds
```

## 🎯 承诺阶段

```rust
fn commit(&mut self) -> Commitment {
    // 1. 计算扩展见证
    let witness = compute_extended_witness(...);
    
    // 2. 转换为比特向量
    let witness_bits = bytes_to_bits(&witness);
    
    // 3. 同态承诺
    let (tags, commitment) = self.hc_sender.commit_to_bits(witness_bits);
    
    self.tags = tags;
    commitment
}
```

## ✅ 一致性证明

```rust
fn commit_prove_consistency(&mut self, challenge: Challenge1) -> Response {
    // 证明承诺的见证与 AES 计算一致
    self.hc_sender.commit_prove_consistency(challenge)
}
```

## 🎲 最终证明

```rust
fn prove(&mut self, challenge: Challenge2) -> Proof {
    // 提供打开部分见证的证明
    // 允许验证者检查 AES 约束
    ...
}
```

## 📍 代码位置

- **Trait**：`src/faest.rs` 第73-103行
- **keygen**：`src/faest.rs` 第35-72行
- **证明者**：`src/faest.rs` 第200-800行
- **验证者**：`src/faest.rs` 第800-1276行

## 📚 总结

FAEST 协议：
- 基于 AES 的零知识证明
- 使用同态承诺
- 证明知道密钥

## 🎓 下一步

协议需要非交互化：
→ `14-Fiat-Shamir变换.md`
