//! The parameters of this kit's commands that name files or folders on disk (KHP A28), declared
//! as typed files in the commands' schemas (`kition_kit::files`): a host lends them, and a
//! headless kit keeps those commands from agents (the host opens, saves and exports documents).

use kition_kit::files::FileParam;

/// Every file or folder parameter of the kit's commands.
pub const FILES: &[FileParam] = &[
    FileParam::read("file.open", "path", &[]),
    FileParam::read("file.revert", "path", &[]),
    FileParam::read("swatch.library.list", "path", &[]),
    FileParam::read("swatch.library.copyToUser", "path", &[]),
    FileParam::read("file.place", "path", &[]),
    FileParam::read("file.place.info", "path", &[]),
    FileParam::read("linkedFile.relink", "path", &[]),
    FileParam::read("pdf.preset.import", "path", &[]),
    FileParam::read("print.presets.import", "path", &[]),
    FileParam::read("perspective.presets.import", "path", &[]),
    FileParam::read("swatch.library.load", "path", &[]),
    FileParam::read("flattener.presets.import", "path", &[]),
    FileParam::read("graphicStyle.loadLibrary", "path", &[]),
    FileParam::read("library.import", "path", &[]),
    FileParam::folder("linkedFile.relink", "folder"),
    FileParam::create("file.export", "path", &[]),
    FileParam::create("file.exportSelection", "path", &[]),
    FileParam::folder("file.exportForScreens", "folder"),
    FileParam::read("file.pdfInfo", "path", &[]),
    FileParam::create("file.exportForOffice", "path", &[]),
    FileParam::write("file.save", "path", &[]),
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
    FileParam::create("file.exportPdf", "path", &[]),
    FileParam::read("file.place.queue", "paths", &[]),
    FileParam::create("linkedFile.unembed", "path", &[]),
    FileParam::create("pdf.preset.export", "path", &[]),
    FileParam::create("file.exportDxf", "path", &[]),
    FileParam::create("file.exportEps", "path", &[]),
    FileParam::folder("file.package", "folder"),
    FileParam::create("file.print", "path", &[]),
    FileParam::read("file.recovery.restore", "file", &[]),
    FileParam::write("file.recovery.discard", "file", &[]),
    FileParam::read("file.dxfInfo", "path", &[]),
    FileParam::create("file.exportForWeb", "path", &[]),
    FileParam::folder("asset.export", "folder"),
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
    const NOT_FILES: &[(&str, &str)] = &[("text.addInPath", "path"), ("file.place.queue", "files")];

    #[test]
    fn every_file_parameter_is_declared() {
        let doc = crate::VectorDocument::create("Untitled.kvector", 64, 64).unwrap_or_else(|error| panic!("{error}"));
        let problems = kition_kit::files::check(&doc.commands(), FILES, NOT_FILES);
        assert!(problems.is_empty(), "{problems:#?}");
    }
}
