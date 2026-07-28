/// Das arrangierte Scheitern eines Seam-Fakes — die gemeinsame Vorbedingung aller Fakes, die den
/// terminalen io-Fehlschlag ihres Seams spielen können. Der **Fehlertyp bleibt der des jeweiligen
/// Seams**: [`check`](Self::check) hüllt die arrangierte Diagnose erst beim Prüfen ein, sodass jeder
/// Fake weiterhin seinen eigenen schmalen Seam-Fehler zurückgibt.
#[derive(Default)]
pub(crate) struct ArrangedFailure(Option<String>);

impl ArrangedFailure {
    /// Schärft das Scheitern mit der gegebenen io-Diagnose.
    pub(crate) fn arm(&mut self, message: &str) {
        self.0 = Some(message.to_string());
    }

    /// Die Vorbedingung — `Ok(())`, solange keines gesetzt ist, sonst die eingehüllte Diagnose.
    pub(crate) fn check<E>(&self, wrap: impl Fn(String) -> E) -> Result<(), E> {
        self.0.clone().map_or(Ok(()), |message| Err(wrap(message)))
    }
}
