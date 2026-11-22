//! The parameters of this kit's commands that name files or folders on disk (KHP A28), declared
//! as typed files in the commands' schemas (`kition_kit::files`): a host lends them, and a
//! headless kit keeps those commands from agents (the host opens, saves and exports documents).

use kition_kit::files::FileParam;

/// Every file or folder parameter of the kit's commands.
pub const FILES: &[FileParam] = &[
    FileParam::read("document.open", "path", &[]),
    FileParam::read("file.revert", "path", &[]),
    FileParam::read("swatch.library.list", "path", &[]),
    FileParam::read("swatch.library.copyToUser", "path", &[]),
    FileParam::read("file.place", "path", &[]),
    FileParam::read("file.place.info", "path", &[]),
    FileParam::read("links.relink", "path", &[]),
    FileParam::read("pdf.preset.import", "path", &[]),
    FileParam::read("print.presets.import", "path", &[]),
    FileParam::read("perspective.presets.import", "path", &[]),
    FileParam::read("swatch.library.load", "path", &[]),
    FileParam::read("flattener.presets.import", "path", &[]),
    FileParam::read("graphicStyle.loadLibrary", "path", &[]),
    FileParam::read("library.import", "path", &[]),
    FileParam::folder("links.relink", "folder"),
    FileParam::create("document.export", "path", &[]),
    FileParam::create("document.exportSelection", "path", &[]),
    FileParam::folder("document.exportForScreens", "folder"),
    FileParam::read("document.pdfInfo", "path", &[]),
    FileParam::create("document.exportForOffice", "path", &[]),
    FileParam::write("document.save", "path", &[]),
    FileParam::create("file.saveAs", "path", &[]),
    FileParam::create("file.saveCopy", "path", &[]),
    FileParam::create("file.saveAsTemplate", "path", &[]),
    FileParam::read("file.newFromTemplate", "path", &[]),
    FileParam::read("color.loadProfile", "path", &[]),
    FileParam::folder("text.findFontFiles", "folder"),
    FileParam::read("text.addFontFiles", "files", &[]),
    FileParam::create("swatch.library.save", "path", &[]),
    FileParam::create("flattener.presets.export", "path", &[]),
    FileParam::create("graphicStyle.saveLibrary", "path", &[]),
    FileParam::create("library.export", "path", &[]),
    FileParam::create("document.exportPdf", "path", &[]),
    FileParam::read("file.place.queue", "paths", &[]),
    FileParam::create("links.unembed", "path", &[]),
    FileParam::create("pdf.preset.export", "path", &[]),
    FileParam::create("document.exportDxf", "path", &[]),
    FileParam::create("document.exportEps", "path", &[]),
    FileParam::folder("file.package", "folder"),
    FileParam::create("file.print", "path", &[]),
    FileParam::read("file.recovery.restore", "file", &[]),
    FileParam::write("file.recovery.discard", "file", &[]),
    FileParam::read("document.dxfInfo", "path", &[]),
    FileParam::create("document.exportForWeb", "path", &[]),
    FileParam::folder("assets.export", "folder"),
    FileParam::create("css.export", "path", &[]),
    FileParam::create("print.presets.export", "path", &[]),
    FileParam::read("plugin.install", "path", &[]),
    FileParam::folder("plugin.reload", "path"),
    FileParam::create("perspective.presets.export", "path", &[]),
];

#[cfg(test)]
mod tests {
    use kition_kit::KitDocument;

    use super::*;

    /// Parameters whose names read like files but are not: ids, property paths, directions,
    /// folders of the kit's own panels, in-memory data.
    const NOT_FILES: &[(&str, &str)] = &[("text.createInPath", "path"), ("file.place.queue", "files")];

    #[test]
    fn every_file_parameter_is_declared() {
        let doc = crate::VectorDocument::create("Untitled.kvector", 64, 64).unwrap_or_else(|error| panic!("{error}"));
        let problems = kition_kit::files::check(&doc.commands(), FILES, NOT_FILES);
        assert!(problems.is_empty(), "{problems:#?}");
    }
}
