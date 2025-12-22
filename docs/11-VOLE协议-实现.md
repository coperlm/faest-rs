# VOLE 协议 - 代码实现

## 🔧 初始化

```rust
impl<F: SmallGF, VC: VecCom> VoleInTheHeadSender for 
    VoleInTheHeadSenderFromVC<F, VC> 
{
    fn new(vole_length: usize, num_repetitions: usize) -> Self {
        Self {
            vole_length,
            num_repetitions,
            state: VoleInTheHeadSenderState::New,
            u: GF2Vector::new(),
            V: Matrix::zeros((vole_length, num_repetitions)),
            decommitment_keys: Vec::new(),
            _phantom_vc: PhantomData,
            _phantom_h: PhantomData,
        }
    }
}
```

## 📨 承诺消息

```rust
fn commit_message(&mut self, message: GF2Vector) -> Commitment {
    assert!(self.state == VoleInTheHeadSenderState::New);
    
    // 1. 存储消息
    self.u = message;
    
    // 2. 为每个重复生成随机矩阵
    let mut commitments = Vec::new();
    
    for j in 0..self.num_repetitions {
        // 生成随机 V 列
        let mut v_col = Vector::zeros(self.vole_length);
        for i in 0..self.vole_length {
            v_col[i] = F::random(&mut rng);
        }
        
        // 存储到矩阵
        for i in 0..self.vole_length {
            self.V[[i, j]] = v_col[i];
        }
        
        // 创建向量承诺
        let mut vec_com = VC::new(self.vole_length);
        let commitment = vec_com.commit(&v_col);
        
        commitments.push(commitment);
        self.decommitment_keys.push(vec_com);
    }
    
    self.state = VoleInTheHeadSenderState::Committed;
    commitments
}
```

## 🎲 一致性检查响应

```rust
fn consistency_check_respond(
    &mut self,
    random_points: Challenge  // tau 个随机点
) -> Response {
    assert!(self.state == VoleInTheHeadSenderState::Committed);
    
    // 计算哈希
    let h_u = hash_bitvector(random_points.view(), &self.u);
    let h_V = hash_matrix(random_points.view(), self.V.view());
    
    self.state = VoleInTheHeadSenderState::RespondedToConsistencyChallenge;
    
    (h_u, h_V)
}
```

## 🔓 去承诺

```rust
fn decommit(&mut self, Deltas: Vector<F>) -> Decommitment {
    assert!(self.state == RespondedToConsistencyChallenge);
    
    let mut decommitments = Vec::new();
    
    for (j, delta) in Deltas.iter().enumerate() {
        // 计算需要打开的位置
        let indices: Vec<usize> = (0..self.vole_length)
            .filter(|&i| {
                let v_delta = self.V[[i, j]] * delta;
                let u_bit = self.u.bits[i];
                
                // 打开不一致的位置
                u_bit != to_bit(v_delta)
            })
            .collect();
        
        // 去承诺这些位置
        let decom = self.decommitment_keys[j].decommit(&indices);
        decommitments.push((indices, decom));
    }
    
    self.state = VoleInTheHeadSenderState::Ready;
    decommitments
}
```

## 🔍 接收方验证

```rust
fn receive_decommitment(&mut self, decommitments: &Decommitment) -> bool {
    for (j, (indices, decom)) in decommitments.iter().enumerate() {
        // 1. 验证向量承诺
        if !self.vec_coms[j].verify(indices, decom) {
            return false;
        }
        
        // 2. 重构矩阵 V
        for &i in indices {
            let v_ij = decom.get_value(i);
            self.V[[i, j]] = v_ij;
        }
        
        // 3. 计算 u 的比特
        for i in 0..self.vole_length {
            let v_delta = self.V[[i, j]] * self.delta[j];
            self.u[i] = to_bit(v_delta);
        }
    }
    
    // 4. 验证一致性响应
    let h_u = hash_bitvector(self.consistency_challenge.view(), &self.u);
    let h_V = hash_matrix(self.consistency_challenge.view(), self.V.view());
    
    h_u == self.consistency_response.0 && 
    h_V == self.consistency_response.1
}
```

## 🎯 关键辅助函数

### 有限域到比特的转换

```rust
fn to_bit<F: SmallGF>(x: F) -> bool {
    // 提取最低位
    (Into::<usize>::into(x) & 1) == 1
}
```

### 生成挑战

```rust
fn consistency_challenge_from_seed(
    num_repetitions: usize,
    seed: [u8; 32]
) -> Vector<F> {
    let mut rng = ChaChaRng::from_seed(seed);
    let mut challenge = Vector::zeros(num_repetitions);
    
    for i in 0..num_repetitions {
        challenge[i] = F::random(&mut rng);
    }
    
    challenge
}
```

## 📊 状态机

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VoleInTheHeadSenderState {
    New,                                 // 初始状态
    Committed,                           // 已承诺
    RespondedToConsistencyChallenge,     // 已响应挑战
    Ready,                               // 已去承诺
}
```

## 🔄 完整流程示例

```rust
// 发送方
let mut sender = VoleInTheHeadSenderFromVC::<GF2p8, VC>::new(128, 16);

// 1. 承诺消息
let message = GF2Vector::from_vec(vec![0xFF; 16]);  // 128 bits
let commitment = sender.commit_message(message);

// 2. 响应挑战
let challenge = generate_challenge();
let response = sender.consistency_check_respond(challenge);

// 3. 最终挑战和去承诺
let deltas = generate_final_challenge();
let decommitment = sender.decommit(deltas);

// 4. 获取输出
let (u, V) = sender.get_output();

// 接收方
let mut receiver = VoleInTheHeadReceiverFromVC::<GF2p8, VC>::new(128, 16);

// 1-2. 接收承诺和生成挑战
receiver.receive_commitment(commitment);
let challenge = receiver.generate_consistency_challenge();

// 3. 接收响应
receiver.store_consistency_response(response);

// 4-5. 生成最终挑战并验证
let deltas = receiver.generate_final_challenge();
let is_valid = receiver.receive_decommitment(&decommitment);

assert!(is_valid);
```

## ⚡ 优化技巧

### 1. 批量承诺
```rust
// 一次承诺多个列
let commitments: Vec<_> = (0..num_repetitions)
    .into_par_iter()  // 并行
    .map(|j| commit_column(j))
    .collect();
```

### 2. 增量哈希
```rust
// 预计算 r 的幂次
let r_powers: Vec<_> = (0..vole_length)
    .scan(F::one(), |pow, _| {
        let result = *pow;
        *pow *= r;
        Some(result)
    })
    .collect();
```

### 3. 稀疏打开
```rust
// 只打开需要的位置（平均 50%）
let indices: Vec<_> = (0..vole_length)
    .filter(|&i| u[i] != to_bit(V[[i, j]] * delta))
    .collect();
```

## 📍 代码位置

- **发送方实现**：`voleith.rs` 第100-350行
- **接收方实现**：`voleith.rs` 第400-650行
- **辅助函数**：`voleith.rs` 第650-659行

## 📚 总结

VOLE 实现的关键：
- **状态管理**：确保正确的调用顺序
- **承诺方案**：使用向量承诺
- **一致性**：通过哈希检查
- **优化**：并行和增量计算

## 🎓 下一步

VOLE 是同态承诺的基础：
→ `12-同态承诺.md`
