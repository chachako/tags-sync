use git2::Signature;

pub type CommitInfo = (Signature<'static>, Signature<'static>, String);
