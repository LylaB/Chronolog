pub mod dynamic;
pub mod flag;
pub mod logical;
pub mod metadata;
pub mod task;

pub use dynamic::*;
pub use flag::*;
pub use logical::*;
pub use metadata::*;
pub use task::*;

use crate::task::ObserverField;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// [`FrameDependency`] describes a dependency for [`DependencyTaskFrame`] which have to be
/// resolved in order to proceed. Dependencies can wrap other dependencies, creating a hierarchy
/// that allows for flexibility on how a developer defines their dependencies
///
/// dependencies can also be manually resolved or unresolved, implementors may use the extension
/// traits [`ResolvableFrameDependency`] and [`UnresolvableFrameDependency`] for these actions
///
/// dependencies can also be disabled and enabled back. Disabled dependencies will be skipped
/// by [`DependencyTaskFrame`].
#[async_trait]
pub trait FrameDependency: Send + Sync + 'static {
    async fn is_resolved(&self) -> bool;
    async fn disable(&self);
    async fn enable(&self);
    async fn is_enabled(&self) -> bool;
}

#[async_trait]
impl<D: FrameDependency + ?Sized> FrameDependency for Arc<D> {
    async fn is_resolved(&self) -> bool {
        self.as_ref().is_resolved().await
    }

    async fn disable(&self) {
        self.as_ref().disable().await
    }

    async fn enable(&self) {
        self.as_ref().enable().await
    }

    async fn is_enabled(&self) -> bool {
        self.as_ref().is_enabled().await
    }
}

/// Represents a resolvable frame dependency, this dependency can be
/// automatically resolved, or it may be manually resolved
#[async_trait]
pub trait ResolvableFrameDependency: FrameDependency {
    async fn resolve(&self);
}

/// Represents an unresolvable frame dependency, this dependency can be
/// manually unresolved to essentially reset its state
#[async_trait]
pub trait UnresolvableFrameDependency: FrameDependency {
    async fn unresolve(&self);
}
