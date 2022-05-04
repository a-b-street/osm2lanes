use super::TagsToLanesMsg;

#[derive(Debug)]
pub struct InferConflict;

impl std::fmt::Display for InferConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "inferred values conflict")
    }
}

impl std::error::Error for InferConflict {}

impl From<InferConflict> for TagsToLanesMsg {
    fn from(_conflict: InferConflict) -> Self {
        TagsToLanesMsg::internal("infer conflict")
    }
}

// TODO: implement try when this is closed: https://github.com/rust-lang/rust/issues/84277
/// A value with various levels of inference
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Infer<T> {
    None,
    Default(T),
    Calculated(T),
    Direct(T),
}

impl<T> Infer<T>
where
    T: PartialEq<T>,
{
    /// `Infer::None`
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Convert any non-`Infer::None` value into `Option::Some`
    pub fn some(self) -> Option<T> {
        match self {
            Self::None => None,
            Self::Default(v) | Self::Calculated(v) | Self::Direct(v) => Some(v),
        }
    }

    /// `Infer::Direct` or `Infer::None` from Option
    pub fn direct(some: Option<T>) -> Self {
        match some {
            None => Self::None,
            Some(v) => Self::Direct(v),
        }
    }
    /// Conditionally replaces value.
    ///
    /// # Replaces
    /// - The same value at a higher confidence
    /// - A different value at a higher confidence
    ///
    /// # Ignores
    /// - The same value at the same confidence
    /// - The same value at a lower confidence
    /// - A different value at a lower confidence
    ///
    /// # Errors
    /// - A different value at the same confidence
    ///
    /// ```
    /// use osm2lanes::transform::Infer;
    /// let mut i = Infer::Default(0);
    /// assert!(i.set(Infer::Direct(1)).is_ok());
    /// assert!(i.set(Infer::Direct(2)).is_err());
    /// assert!(i.set(Infer::Default(3)).is_ok());
    /// assert!(i.set(Infer::None).is_ok());
    /// ```
    pub fn set(&mut self, value: Infer<T>) -> Result<(), InferConflict> {
        match (self, value) {
            (_, Infer::None)
            | (Infer::Direct(_), Infer::Calculated(_) | Infer::Default(_))
            | (Infer::Calculated(_), Infer::Default(_)) => Ok(()),
            (swap @ Infer::None, value)
            | (swap @ Infer::Default(_), value @ (Infer::Direct(_) | Infer::Calculated(_)))
            | (swap @ Infer::Calculated(_), value @ Infer::Direct(_)) => {
                *swap = value;
                Ok(())
            },
            (Infer::Default(left), Infer::Default(right))
            | (Infer::Calculated(left), Infer::Calculated(right))
            | (Infer::Direct(left), Infer::Direct(right)) => {
                if left == &right {
                    Ok(())
                } else {
                    Err(InferConflict)
                }
            },
        }
    }

    /// Analogous to `Option::map`
    pub fn map<U, F>(self, f: F) -> Infer<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Infer::None => Infer::None,
            Infer::Default(x) => Infer::Default(f(x)),
            Infer::Calculated(x) => Infer::Calculated(f(x)),
            Infer::Direct(x) => Infer::Direct(f(x)),
        }
    }

    /// If `Infer::None`, replaces with `Infer::Default(d)`
    #[must_use]
    pub fn or_default(self, d: T) -> Self {
        match self {
            Infer::None => Infer::Default(d),
            other => other,
        }
    }
}

impl<T> Default for Infer<T> {
    fn default() -> Self {
        Self::None
    }
}

impl<T> From<Option<T>> for Infer<T> {
    fn from(some: Option<T>) -> Self {
        match some {
            Some(val) => Self::Direct(val),
            None => Self::None,
        }
    }
}
