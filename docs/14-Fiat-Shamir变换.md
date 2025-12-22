# Fiat-Shamir 变换

## 🎯 从交互到非交互

**问题**：交互式证明需要验证者在线生成挑战

**解决**：用哈希函数模拟验证者

## 🔄 变换原理

### 交互式（需要往返）

```
证明者                验证者
------                ------
承诺 C     ------>
           <------    挑战 e (随机)
响应 z     ------>
                      验证(C, e, z)
```

### 非交互式（Fiat-Shamir 后）

```
证明者                验证者
------                ------
承诺 C
挑战 e = H(C || msg)  
响应 z
签名 = (C, z)  ---->  
                      e' = H(C || msg)
                      验证(C, e', z)
```

## 🏗️ 签名结构

```rust
pub struct FsSignature<P: Prover> {
    pub commitment: P::Commitment,
    pub response: P::Response,
    pub proof: P::Proof,
    pub decommitment: P::Decommitment,
}
```

## ✍️ 签名者

```rust
pub struct FsSigner<P: Prover, V: Verifier> {
    prover: P,
    verifier: V,
}

impl<P: Prover, V: Verifier> Signer for FsSigner<P, V> {
    fn sign(&self, message: &[u8]) -> FsSignature<P> {
        // 1. 承诺
        let commitment = self.prover.commit();
        
        // 2. 生成挑战1（用哈希）
        let challenge1 = hash(commitment || message);
        let response = self.prover.commit_prove_consistency(challenge1);
        
        // 3. 生成挑战2
        let challenge2 = hash(commitment || response || message);
        let proof = self.prover.prove(challenge2);
        
        // 4. 生成选择
        let choice = hash(commitment || proof || message);
        let decommitment = self.prover.transfer(choice);
        
        FsSignature {
            commitment,
            response,
            proof,
            decommitment,
        }
    }
}
```

## 🔍 验证者

```rust
pub struct FsVerifier<P: Prover, V: Verifier> {
    verifier: V,
    _phantom: PhantomData<P>,
}

impl<P: Prover, V: Verifier> SignatureVerifier for FsVerifier<P, V> {
    fn verify(&self, signature: &FsSignature<P>, message: &[u8]) -> bool {
        // 1. 接收承诺
        self.verifier.receive_commitment(signature.commitment);
        
        // 2. 重新计算挑战1
        let challenge1 = hash(signature.commitment || message);
        
        // 3. 接收响应
        self.verifier.receive_response(signature.response);
        
        // 4. 重新计算挑战2
        let challenge2 = hash(signature.commitment || signature.response || message);
        
        // 5. 验证证明
        self.verifier.verify(
            signature.proof,
            signature.decommitment,
            challenge2
        )
    }
}
```

## 🔐 哈希链

```
H0 = H(commitment || message)
H1 = H(commitment || response || message)
H2 = H(commitment || proof || message)
...
```

每个哈希输出用作下一阶段的挑战。

## 📊 序列化

```rust
impl<P: Prover> FsSignature<P> {
    pub fn to_bytes(&self) -> Vec<u8> {
        // 使用 bincode 序列化
        bincode::encode_to_vec(self, config()).unwrap()
    }
    
    pub fn from_bytes(bytes: &[u8]) -> Self {
        bincode::decode_from_slice(bytes, config()).unwrap().0
    }
}
```

## 🎯 类型别名

在 `lib.rs` 中定义的便捷类型：

```rust
pub type FaestSignature<F> = 
    fiat_shamir::FsSignature<FaestProver<F>>;

pub type FaestSigner<F> = 
    fiat_shamir::FsSigner<FaestProver<F>, FaestVerifier<F>>;

pub type FaestSignatureVerifier<F> = 
    fiat_shamir::FsVerifier<FaestProver<F>, FaestVerifier<F>>;
```

## 💻 使用示例

```rust
use faest::{keygen, FaestSigner, FaestSignatureVerifier};
use faest::aes::Aes;
use faest::gf2psmall::GF2p8;

// 1. 生成密钥
let (sk, pk) = keygen(Aes::Aes128);

// 2. 创建签名者
let signer = FaestSigner::<GF2p8>::new(sk, pk);

// 3. 签名
let message = b"Hello, world!";
let signature = signer.sign(message);

// 4. 创建验证者
let verifier = FaestSignatureVerifier::<GF2p8>::new(Aes::Aes128, pk);

// 5. 验证
let is_valid = verifier.verify(&signature, message);
assert!(is_valid);
```

## 📍 代码位置

- **签名结构**：`src/fiat_shamir.rs` 第30-40行
- **签名者**：`src/fiat_shamir.rs` 第100-200行
- **验证者**：`src/fiat_shamir.rs` 第200-300行

## 📚 总结

Fiat-Shamir 变换：
- 消除交互
- 哈希作为随机预言机
- 安全且高效

## 🎓 下一步

看完整的端到端示例：
→ `15-完整流程示例.md`
