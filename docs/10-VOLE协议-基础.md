# VOLE 协议 - 基础原理

## 🎯 什么是 VOLE？

**VOLE** = Vector Oblivious Linear Evaluation（向量不经意线性求值）

两方安全计算线性函数，互不知道对方的输入。

## 📐 数学形式

```
发送方有：向量 v ∈ F^n
接收方有：标量 Δ ∈ F

目标：接收方得到 u = v · Δ
      但发送方不知道 Δ
```

其中 `·` 是标量乘法：
```
u[i] = v[i] × Δ
```

## 🎭 角色和交互

### 发送方（Sender）

```rust
pub trait VoleInTheHeadSender {
    // 1. 承诺消息（比特向量）
    fn commit_message(&mut self, message: GF2Vector) -> Commitment;
    
    // 2. 响应一致性挑战
    fn consistency_check_respond(&mut self, 
        random_points: Challenge
    ) -> Response;
    
    // 3. 提供最终证明
    fn decommit(&mut self, Deltas: Vector<Field>) -> Decommitment;
    
    // 4. 获取输出
    fn get_output(&self) -> (&GF2View, MatrixView<Field>);
}
```

### 接收方（Receiver）

```rust
pub trait VoleInTheHeadReceiver {
    // 1. 接收承诺
    fn receive_commitment(&mut self, commitment: Commitment);
    
    // 2. 生成挑战
    fn generate_consistency_challenge(&mut self) -> Challenge;
    
    // 3. 接收响应
    fn store_consistency_response(&mut self, response: Response);
    
    // 4. 生成最终挑战
    fn generate_final_challenge(&mut self) -> Vector<Field>;
    
    // 5. 验证并获取输出
    fn receive_decommitment(&mut self, 
        decommitment: &Decommitment
    ) -> bool;
}
```

## 🔄 交互流程

```
发送方                                    接收方
------                                    ------
1. 选择随机 v
   承诺 v
            ----[Commitment]---->
                                          2. 接收承诺
                                          
                                          3. 生成随机挑战 r
            <----[Challenge r]----
            
4. 计算响应
   h = hash(v, r)
            ----[Response h]---->
                                          5. 存储响应
                                          
                                          6. 生成最终挑战 Δ
            <----[Challenge Δ]----
            
7. 计算 u = v · Δ
   打开承诺
            ----[Decommitment]---->
                                          8. 验证
                                             检查 h 正确
                                             得到 (u, v)
```

## 🧮 "In-the-Head" 技巧

**关键思想**：把两方协议在"脑子里"模拟！

```
正常 VOLE：两个实体在通信
VOLE-in-the-Head：一个人同时扮演两个角色

证明者：
  - 扮演发送方
  - 扮演接收方
  - 用 Fiat-Shamir 生成挑战

验证者：
  - 重新计算挑战
  - 检查一致性
```

## 📊 数据结构

### 发送方状态

```rust
pub struct VoleInTheHeadSenderFromVC<F, VC> {
    vole_length: usize,          // 向量长度
    num_repetitions: usize,      // 重复次数（安全参数）
    state: SenderState,          // 状态机
    u: GF2Vector,               // 比特向量 u
    V: Matrix<F>,               // 有限域矩阵 V
    decommitment_keys: Vec<...>, // 去承诺密钥
}
```

### 接收方状态

```rust
pub struct VoleInTheHeadReceiverFromVC<F, VC> {
    vole_length: usize,
    num_repetitions: usize,
    state: ReceiverState,
    consistency_challenge: Challenge,
    consistency_response: Response,
    delta: Vector<F>,           // 最终挑战
    u: Vector<F>,              // 输出向量
    V: Matrix<F>,              // 输出矩阵
}
```

## 🔐 安全性

### 一致性检查

确保发送方诚实：

```rust
// 发送方承诺后，不能更改
let h_u = hash_bitvector(challenge, &u);
let h_V = hash_matrix(challenge, &V);

// 必须满足：h_u 对应 h_V
```

### 重复放大

```rust
// 重复 num_repetitions 次
// 每次作弊概率：1/|F|
// 总作弊概率：(1/|F|)^num_repetitions

// 例如：F = GF(2^8), 重复 16 次
// 作弊概率 < (1/256)^16 ≈ 2^-128
```

## 🎯 输出格式

### 发送方输出

```rust
(&u, V) where:
  u: GF2Vector of length n
  V: Matrix<F> of shape (n, tau)
  tau = num_repetitions
```

### 接收方输出

```rust
(delta, V) where:
  delta: Vector<F> of length tau
  V: Matrix<F> of shape (n, tau)
```

### 关系验证

```rust
// 对于每个 i:
// u[i] == (V[i, :] · delta) 在 GF(2) 中

// 即：比特 u[i] 等于有限域内积转换到 GF(2)
```

## 💡 为什么这样设计？

1. **比特 vs 有限域**
   - u 是比特：小，快速传输
   - V 是有限域：支持代数运算

2. **矩阵形式**
   - 每行是一个 VOLE 实例
   - tau 列提供冗余和安全性

3. **In-the-Head**
   - 非交互化
   - 可转换为签名

## 📍 代码位置

- **Trait定义**：`src/voleith.rs` 第17-60行
- **发送方**：`src/voleith.rs` 第100-400行
- **接收方**：`src/voleith.rs` 第400-659行

## 📚 总结

**VOLE-in-the-Head**：
- 安全两方计算
- 在"脑中"模拟
- 基于向量承诺
- 支持非交互化

## 🎓 下一步

理解了原理，现在看具体实现：
→ `11-VOLE协议-实现.md`
