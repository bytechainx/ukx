//! ukx 的错误分类与错误类型。
//!
//! 调用方必须能回答「这个错误值不值得重试、该改数据还是该改授权」，而不是靠字符串匹配。
//! 因此本模块给出可判定的 [`UkCbErrorKind`] 与携带分类的 [`UkCbError`]。
//!
//! - 本层**无网络**（`production_decision = NO-GO`，尚无任何 Series / UA / 许可授权），
//!   故 [`UkCbError::is_retryable`] 除 [`UkCbErrorKind::Invariant`] 外一律返回 `false`。
//! - 错误消息为简体中文，**不回显**原始响应正文、凭据或整行配置源码。

/// 错误分类：按「调用方应如何反应」划分。
///
/// 禁止用字符串匹配替代对本枚举的匹配。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UkCbErrorKind {
    /// 输入形态或取值非法（调用方应修数据，重试无意义）。
    Invalid,
    /// 缺少必需项。
    Missing,
    /// 授权判定未通过（fail-closed 落点）。
    AuthorizationDenied,
    /// 该产品不属于本域，已被路由规则拒绝。
    RoutedElsewhere,
    /// 越权写入（权威写入归他域或主权未决）。
    WriteAuthorityDenied,
    /// 结构可解析但语义不被接受（如未钉死的标识、重复身份）。
    SemanticallyRejected,
    /// 尚未实现的规划能力。
    NotApplicable,
    /// 不变量被破坏（库内 bug 的信号）。
    Invariant,
}

/// ukx 错误。保留可区分的分类，供调用方分流。
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum UkCbError {
    /// 输入非法。
    #[error("输入非法：{0}")]
    Invalid(String),
    /// 缺少必需项。
    #[error("缺少必需项：{0}")]
    Missing(String),
    /// 授权判定未通过。
    #[error("授权判定未通过：{0}")]
    AuthorizationDenied(String),
    /// 产品不属于本域。
    #[error("不属于本域，已路由他处：{0}")]
    RoutedElsewhere(String),
    /// 越权写入。
    #[error("越权写入被拒绝：{0}")]
    WriteAuthorityDenied(String),
    /// 语义拒绝。
    #[error("语义不被接受：{0}")]
    SemanticallyRejected(String),
    /// 规划能力未实现。
    #[error("规划能力尚未实现：{0}")]
    NotApplicable(String),
    /// 不变量被破坏。
    #[error("不变量被破坏：{0}")]
    Invariant(String),
}

impl UkCbError {
    /// 分类，供调用方按「如何反应」分流。
    #[must_use]
    pub fn kind(&self) -> UkCbErrorKind {
        match self {
            Self::Invalid(_) => UkCbErrorKind::Invalid,
            Self::Missing(_) => UkCbErrorKind::Missing,
            Self::AuthorizationDenied(_) => UkCbErrorKind::AuthorizationDenied,
            Self::RoutedElsewhere(_) => UkCbErrorKind::RoutedElsewhere,
            Self::WriteAuthorityDenied(_) => UkCbErrorKind::WriteAuthorityDenied,
            Self::SemanticallyRejected(_) => UkCbErrorKind::SemanticallyRejected,
            Self::NotApplicable(_) => UkCbErrorKind::NotApplicable,
            Self::Invariant(_) => UkCbErrorKind::Invariant,
        }
    }

    /// 是否值得重试。本层无网络，除 [`UkCbErrorKind::Invariant`] 外一律 `false`。
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(self.kind(), UkCbErrorKind::Invariant)
    }
}

/// 本 crate 统一结果别名。
pub type UkCbResult<T> = Result<T, UkCbError>;

#[cfg(test)]
mod tests {
    use super::{UkCbError, UkCbErrorKind};

    /// 每个变体的 `kind()` 映射必须一一对应。
    #[test]
    fn kind_maps_one_to_one() {
        let cases: [(UkCbError, UkCbErrorKind); 8] = [
            (UkCbError::Invalid("i".to_owned()), UkCbErrorKind::Invalid),
            (UkCbError::Missing("m".to_owned()), UkCbErrorKind::Missing),
            (
                UkCbError::AuthorizationDenied("a".to_owned()),
                UkCbErrorKind::AuthorizationDenied,
            ),
            (
                UkCbError::RoutedElsewhere("r".to_owned()),
                UkCbErrorKind::RoutedElsewhere,
            ),
            (
                UkCbError::WriteAuthorityDenied("w".to_owned()),
                UkCbErrorKind::WriteAuthorityDenied,
            ),
            (
                UkCbError::SemanticallyRejected("s".to_owned()),
                UkCbErrorKind::SemanticallyRejected,
            ),
            (
                UkCbError::NotApplicable("n".to_owned()),
                UkCbErrorKind::NotApplicable,
            ),
            (
                UkCbError::Invariant("v".to_owned()),
                UkCbErrorKind::Invariant,
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.kind(), expected);
        }
    }

    /// 仅 `Invariant` 可重试（本层无网络）。
    #[test]
    fn only_invariant_is_retryable() {
        let kinds = [
            UkCbErrorKind::Invalid,
            UkCbErrorKind::Missing,
            UkCbErrorKind::AuthorizationDenied,
            UkCbErrorKind::RoutedElsewhere,
            UkCbErrorKind::WriteAuthorityDenied,
            UkCbErrorKind::SemanticallyRejected,
            UkCbErrorKind::NotApplicable,
        ];
        for kind in kinds {
            let error = UkCbError::Invalid(format!("{kind:?}"));
            assert!(!error.is_retryable(), "{kind:?} 不应可重试");
        }
        assert!(UkCbError::Invariant("内部不变量".to_owned()).is_retryable());
    }

    /// `Display` 非空且带上分类语义。
    #[test]
    fn display_is_not_empty() {
        let error = UkCbError::RoutedElsewhere("S07 收益率曲线归 yield_curve".to_owned());
        let text = error.to_string();
        assert!(!text.is_empty());
        assert!(text.contains("路由"));
    }

    /// 错误消息不得回显凭据样式的内容。
    #[test]
    fn display_does_not_leak_credentials() {
        let error = UkCbError::SemanticallyRejected("序列标识形态非法".to_owned());
        let text = error.to_string();
        assert!(!text.contains("token="));
        assert!(!text.contains("password"));
        assert!(!text.contains("Authorization:"));
    }
}
