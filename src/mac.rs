use syntax::ast;
use syntax::codemap::{self, DUMMY_SP, Span, respan};
use syntax::ext::base::ExtCtxt;
use syntax::ext::expand;
use syntax::ext::quote::rt::ToTokens;
use syntax::feature_gate::GatedCfgAttr;
use syntax::parse::ParseSess;
use syntax::ptr::P;

use expr::ExprBuilder;
use invoke::{Invoke, Identity};
use name::ToName;

/// A Builder for macro invocations.
///
/// Note that there are no commas added between args, as otherwise
/// that macro invocations that could be expressed would be limited.
/// You will need to add all required symbols with `with_arg` or
/// `with_argss`.
pub struct MacBuilder<F=Identity> {
    callback: F,
    span: Span,
    tokens: Vec<ast::TokenTree>,
    path: Option<ast::Path>,
}

impl MacBuilder {
    pub fn new() -> Self {
        MacBuilder::with_callback(Identity)
    }
}

impl<F> MacBuilder<F>
    where F: Invoke<ast::Mac>
{
    pub fn with_callback(callback: F) -> Self {
        MacBuilder {
            callback: callback,
            span: DUMMY_SP,
            tokens: vec![],
            path: None,
        }
    }

    pub fn span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }

    pub fn path(mut self, path: ast::Path) -> Self {
        self.path = Some(path);
        self
    }

    pub fn build(self) -> F::Result {
        let mac = ast::Mac_ {
            path: self.path.expect("No path set for macro"),
            tts: self.tokens,
            ctxt: ast::EMPTY_CTXT,
        };
        self.callback.invoke(respan(self.span, mac))
    }

    pub fn with_args<I, T>(self, iter: I) -> Self
        where I: IntoIterator<Item=T>, T: ToTokens
    {
        iter.into_iter().fold(self, |self_, expr| self_.with_arg(expr))
    }

    pub fn with_arg<T>(mut self, expr: T) -> Self
        where T: ToTokens
    {
        let parse_sess = ParseSess::new();
        let mut feature_gated_cfgs = Vec::new();
        let cx = make_ext_ctxt(&parse_sess, &mut feature_gated_cfgs);
        let tokens = expr.to_tokens(&cx);
        assert!(tokens.len() == 1);
        self.tokens.push(tokens[0].clone());
        self
    }

    pub fn expr(self) -> ExprBuilder<Self> {
        ExprBuilder::with_callback(self)
    }

}

impl<F> Invoke<P<ast::Expr>> for MacBuilder<F>
    where F: Invoke<ast::Mac>,
{
    type Result = Self;

    fn invoke(self, expr: P<ast::Expr>) -> Self {
        self.with_arg(expr)
    }
}

fn make_ext_ctxt<'a>(sess: &'a ParseSess,
                     feature_gated_cfgs: &'a mut Vec<GatedCfgAttr>) -> ExtCtxt<'a> {
    let info = codemap::ExpnInfo {
        call_site: codemap::DUMMY_SP,
        callee: codemap::NameAndSpan {
            format: codemap::MacroAttribute("test".to_name()),
            allow_internal_unstable: false,
            span: None
        }
    };

    let cfg = vec![];
    let ecfg = expand::ExpansionConfig::default(String::new());

    let mut cx = ExtCtxt::new(&sess, cfg, ecfg, feature_gated_cfgs);
    cx.bt_push(info);

    cx
}
