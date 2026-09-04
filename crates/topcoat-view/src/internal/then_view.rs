use std::{
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;
use topcoat_core::error::Result;

use crate::{View, ViewFirst, ViewSwap};

pin_project! {
    /// A [`View`] built from a [`Future`] that resolves to one.
    ///
    /// The view awaits the future first and then polls the view it resolved
    /// to in place. A component invocation becomes one: the component's
    /// body is a future returning its view.
    ///
    /// Wrapping a fallible async body turns the body into a view; the
    /// body's error surfaces when the view renders. This is how a view is
    /// built by hand when the code around it is not a component:
    ///
    /// ```rust
    /// use topcoat::{context::Cx, view::{BoxView, ThenView, view}};
    ///
    /// // The title arrives after the view is built; the view resolves it
    /// // when it renders.
    /// fn page(cx: &Cx) -> BoxView<'_> {
    ///     Box::pin(ThenView::new(async move {
    ///         let title = fetch_title().await;
    ///         Ok(view! { cx => <h1>(title)</h1> })
    ///     }))
    /// }
    ///
    /// async fn fetch_title() -> String {
    ///     String::from("Dashboard")
    /// }
    /// ```
    #[project = ThenViewProj]
    pub enum ThenView<F, V> {
        Future { #[pin] future: F },
        View { #[pin] view: V },
    }
}

impl<F, V> ThenView<F, V>
where
    F: Future<Output = Result<V>>,
{
    #[must_use]
    pub fn new(future: F) -> Self {
        Self::Future { future }
    }
}

impl<F, V> View for ThenView<F, V>
where
    F: Future<Output = Result<V>> + Send,
    V: View,
{
    fn poll_first(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<ViewFirst>> {
        loop {
            match self.as_mut().project() {
                ThenViewProj::Future { future } => {
                    let view = ready!(future.poll(cx))?;
                    self.as_mut().set(Self::View { view });
                }
                ThenViewProj::View { view } => return view.poll_first(cx),
            }
        }
    }

    fn poll_swap(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Option<ViewSwap>>> {
        match self.project() {
            ThenViewProj::Future { .. } => panic!(
                "called `.poll_swap` on a `ThenView` that has not yet emitted any `First` content"
            ),
            ThenViewProj::View { view } => view.poll_swap(cx),
        }
    }
}
