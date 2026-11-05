//! The words of the window in the eight languages of the repository (English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal).

/// The languages of the window, in the order the lists of words below follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
    De,
    Fr,
    It,
    Ja,
    Zh,
    Pt,
}

impl Lang {
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::De,
        Lang::Fr,
        Lang::It,
        Lang::Ja,
        Lang::Zh,
        Lang::Pt,
    ];

    /// The two letters of the language (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`).
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::It => "it",
            Lang::Ja => "ja",
            Lang::Zh => "zh",
            Lang::Pt => "pt",
        }
    }

    /// The name of the language in the language itself.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::It => "Italiano",
            Lang::Ja => "日本語",
            Lang::Zh => "中文",
            Lang::Pt => "Português",
        }
    }

    /// The language of a code such as `es`, `es_ES.UTF-8` or `pt-PT`.
    pub fn from_code(code: &str) -> Option<Lang> {
        let two: String = code
            .chars()
            .take(2)
            .collect::<String>()
            .to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| l.code() == two)
    }

    /// The language of the person from the variables of the system; English when it is none of the eight.
    pub fn from_env() -> Lang {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                if !v.is_empty() {
                    return Lang::from_code(&v).unwrap_or(Lang::En);
                }
            }
        }
        Lang::En
    }
}

/// Puts the values in the places `{0}`, `{1}`... of a text.
pub fn fill(template: &str, values: &[&str]) -> String {
    let mut out = template.to_string();
    for (k, v) in values.iter().enumerate() {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Every word and sentence of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    Language,
    Fit,
    Cancel,
    Up,
    AllFiles,
    EmptyFolder,
    FileName,
    Export,
    ZoomHint,
    NoFile,
    Ready,
    Failed,
    Written,
    SameFile,
    BadValue,
    Stale,
    CursorPlain,
    OpenImage,
    Compute,
    ExportGcode,
    TitleOpenImage,
    TitleExport,
    SectionFile,
    SectionMode,
    SectionImage,
    SectionLaser,
    SectionCalibration,
    SectionResult,
    ModeEngrave,
    ModeCalibrate,
    WidthMm,
    DotMm,
    Dither,
    DitherFs,
    DitherAtkinson,
    DitherThreshold,
    Threshold,
    Feed,
    Power,
    SMax,
    Overscan,
    OverscanAuto,
    Accel,
    OriginX,
    OriginY,
    Powers,
    Feeds,
    LegendBurn,
    LegendTravel,
    LegendLow,
    LegendHigh,
    DropHint,
    NoVectors,
    InfoImage,
    ImageOpened,
    Engraved,
    Calibrated,
    NeedImage,
    Truncated,
}

impl Msg {
    pub const ALL: [Msg; 59] = [
        Msg::Language,
        Msg::Fit,
        Msg::Cancel,
        Msg::Up,
        Msg::AllFiles,
        Msg::EmptyFolder,
        Msg::FileName,
        Msg::Export,
        Msg::ZoomHint,
        Msg::NoFile,
        Msg::Ready,
        Msg::Failed,
        Msg::Written,
        Msg::SameFile,
        Msg::BadValue,
        Msg::Stale,
        Msg::CursorPlain,
        Msg::OpenImage,
        Msg::Compute,
        Msg::ExportGcode,
        Msg::TitleOpenImage,
        Msg::TitleExport,
        Msg::SectionFile,
        Msg::SectionMode,
        Msg::SectionImage,
        Msg::SectionLaser,
        Msg::SectionCalibration,
        Msg::SectionResult,
        Msg::ModeEngrave,
        Msg::ModeCalibrate,
        Msg::WidthMm,
        Msg::DotMm,
        Msg::Dither,
        Msg::DitherFs,
        Msg::DitherAtkinson,
        Msg::DitherThreshold,
        Msg::Threshold,
        Msg::Feed,
        Msg::Power,
        Msg::SMax,
        Msg::Overscan,
        Msg::OverscanAuto,
        Msg::Accel,
        Msg::OriginX,
        Msg::OriginY,
        Msg::Powers,
        Msg::Feeds,
        Msg::LegendBurn,
        Msg::LegendTravel,
        Msg::LegendLow,
        Msg::LegendHigh,
        Msg::DropHint,
        Msg::NoVectors,
        Msg::InfoImage,
        Msg::ImageOpened,
        Msg::Engraved,
        Msg::Calibrated,
        Msg::NeedImage,
        Msg::Truncated,
    ];

    /// The words in the order English, Spanish, German, French, Italian, Japanese, Chinese, Portuguese.
    fn words(self) -> [&'static str; 8] {
        match self {
            Msg::Language => [
                "Language",
                "Idioma",
                "Sprache",
                "Langue",
                "Lingua",
                "言語",
                "语言",
                "Idioma",
            ],
            Msg::Fit => [
                "Fit to window",
                "Ajustar a la ventana",
                "An Fenster anpassen",
                "Ajuster à la fenêtre",
                "Adatta alla finestra",
                "ウィンドウに合わせる",
                "适应窗口",
                "Ajustar à janela",
            ],
            Msg::Cancel => [
                "Cancel",
                "Cancelar",
                "Abbrechen",
                "Annuler",
                "Annulla",
                "キャンセル",
                "取消",
                "Cancelar",
            ],
            Msg::Up => [
                "Up",
                "Subir",
                "Hoch",
                "Dossier parent",
                "Su",
                "上へ",
                "上一级",
                "Subir",
            ],
            Msg::AllFiles => [
                "All files",
                "Todos los ficheros",
                "Alle Dateien",
                "Tous les fichiers",
                "Tutti i file",
                "すべてのファイル",
                "所有文件",
                "Todos os ficheiros",
            ],
            Msg::EmptyFolder => [
                "No files of this kind here",
                "No hay ficheros de este tipo aquí",
                "Hier gibt es keine Dateien dieser Art",
                "Aucun fichier de ce type ici",
                "Nessun file di questo tipo qui",
                "この種類のファイルはありません",
                "此处没有这类文件",
                "Não há ficheiros deste tipo aqui",
            ],
            Msg::FileName => [
                "File name",
                "Nombre del fichero",
                "Dateiname",
                "Nom du fichier",
                "Nome del file",
                "ファイル名",
                "文件名",
                "Nome do ficheiro",
            ],
            Msg::Export => [
                "Export",
                "Exportar",
                "Exportieren",
                "Exporter",
                "Esporta",
                "書き出し",
                "导出",
                "Exportar",
            ],
            Msg::ZoomHint => [
                "Wheel: zoom - drag: move - double click: fit",
                "Rueda: zoom - arrastrar: mover - doble clic: ajustar",
                "Rad: Zoom - Ziehen: verschieben - Doppelklick: anpassen",
                "Molette : zoom - glisser : déplacer - double clic : ajuster",
                "Rotella: zoom - trascina: sposta - doppio clic: adatta",
                "ホイール: ズーム - ドラッグ: 移動 - ダブルクリック: 全体表示",
                "滚轮：缩放 - 拖动：平移 - 双击：适应窗口",
                "Roda: zoom - arrastar: mover - duplo clique: ajustar",
            ],
            Msg::NoFile => [
                "(no file)",
                "(sin fichero)",
                "(keine Datei)",
                "(aucun fichier)",
                "(nessun file)",
                "（ファイルなし）",
                "（无文件）",
                "(sem ficheiro)",
            ],
            Msg::Ready => [
                "Ready",
                "Listo",
                "Bereit",
                "Prêt",
                "Pronto",
                "準備完了",
                "就绪",
                "Pronto",
            ],
            Msg::Failed => [
                "{0}",
                "{0}",
                "{0}",
                "{0}",
                "{0}",
                "{0}",
                "{0}",
                "{0}",
            ],
            Msg::Written => [
                "Written: {0}",
                "Escrito: {0}",
                "Geschrieben: {0}",
                "Écrit : {0}",
                "Scritto: {0}",
                "書き込みました: {0}",
                "已写入：{0}",
                "Escrito: {0}",
            ],
            Msg::SameFile => [
                "{0}: the output file is also an input file; write to another file",
                "{0}: el fichero de salida es también un fichero de entrada; escribe en otro",
                "{0}: Die Ausgabedatei ist auch eine Eingabedatei; schreiben Sie in eine andere Datei",
                "{0} : le fichier de sortie est aussi un fichier d'entrée ; écrivez dans un autre fichier",
                "{0}: il file di uscita è anche un file di ingresso; scrivi su un altro file",
                "{0}: 出力ファイルが入力ファイルと同じです。別のファイルに書き込んでください",
                "{0}：输出文件同时也是输入文件，请写入另一个文件",
                "{0}: o ficheiro de saída é também um ficheiro de entrada; escreva noutro ficheiro",
            ],
            Msg::BadValue => [
                "{0}: this value is not valid",
                "{0}: este valor no es válido",
                "{0}: Dieser Wert ist ungültig",
                "{0} : cette valeur n'est pas valide",
                "{0}: questo valore non è valido",
                "{0}: この値は無効です",
                "{0}：该值无效",
                "{0}: este valor não é válido",
            ],
            Msg::Stale => [
                "The settings changed: compute again",
                "Los ajustes cambiaron: vuelve a calcular",
                "Die Einstellungen haben sich geändert: neu berechnen",
                "Les réglages ont changé : recalculez",
                "Le impostazioni sono cambiate: calcola di nuovo",
                "設定が変わりました。もう一度計算してください",
                "设置已更改：请重新计算",
                "As definições mudaram: calcule outra vez",
            ],
            Msg::CursorPlain => [
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
            ],
            Msg::OpenImage => [
                "Open image...",
                "Abrir imagen...",
                "Bild öffnen...",
                "Ouvrir une image...",
                "Apri immagine...",
                "画像を開く...",
                "打开图像...",
                "Abrir imagem...",
            ],
            Msg::Compute => [
                "Compute the preview",
                "Calcular la vista previa",
                "Vorschau berechnen",
                "Calculer l'aperçu",
                "Calcola l'anteprima",
                "プレビューを計算",
                "计算预览",
                "Calcular a pré-visualização",
            ],
            Msg::ExportGcode => [
                "Export G-code...",
                "Exportar G-code...",
                "G-Code exportieren...",
                "Exporter le G-code...",
                "Esporta G-code...",
                "G コードを書き出し...",
                "导出 G 代码...",
                "Exportar G-code...",
            ],
            Msg::TitleOpenImage => [
                "Open a PNG or PGM picture",
                "Abrir una imagen PNG o PGM",
                "PNG- oder PGM-Bild öffnen",
                "Ouvrir une image PNG ou PGM",
                "Apri un'immagine PNG o PGM",
                "PNG または PGM 画像を開く",
                "打开 PNG 或 PGM 图像",
                "Abrir uma imagem PNG ou PGM",
            ],
            Msg::TitleExport => [
                "Write the G-code",
                "Escribir el G-code",
                "G-Code schreiben",
                "Écrire le G-code",
                "Scrivi il G-code",
                "G コードを書き込む",
                "写入 G 代码",
                "Escrever o G-code",
            ],
            Msg::SectionFile => [
                "Picture",
                "Imagen",
                "Bild",
                "Image",
                "Immagine",
                "画像",
                "图像",
                "Imagem",
            ],
            Msg::SectionMode => [
                "What to make",
                "Qué hacer",
                "Was erzeugt wird",
                "Que faire",
                "Cosa fare",
                "作るもの",
                "要生成的内容",
                "O que fazer",
            ],
            Msg::SectionImage => [
                "Dots",
                "Puntos",
                "Punkte",
                "Points",
                "Punti",
                "ドット",
                "点",
                "Pontos",
            ],
            Msg::SectionLaser => [
                "Laser",
                "Láser",
                "Laser",
                "Laser",
                "Laser",
                "レーザー",
                "激光",
                "Laser",
            ],
            Msg::SectionCalibration => [
                "Calibration grid",
                "Rejilla de calibración",
                "Kalibrierraster",
                "Grille d'étalonnage",
                "Griglia di calibrazione",
                "校正グリッド",
                "校准网格",
                "Grelha de calibração",
            ],
            Msg::SectionResult => [
                "Result",
                "Resultado",
                "Ergebnis",
                "Résultat",
                "Risultato",
                "結果",
                "结果",
                "Resultado",
            ],
            Msg::ModeEngrave => [
                "Engrave the picture",
                "Grabar la imagen",
                "Bild gravieren",
                "Graver l'image",
                "Incidere l'immagine",
                "画像を彫刻",
                "雕刻图像",
                "Gravar a imagem",
            ],
            Msg::ModeCalibrate => [
                "Calibration grid",
                "Rejilla de calibración",
                "Kalibrierraster",
                "Grille d'étalonnage",
                "Griglia di calibrazione",
                "校正グリッド",
                "校准网格",
                "Grelha de calibração",
            ],
            Msg::WidthMm => [
                "Width of the engraving (mm)",
                "Ancho del grabado (mm)",
                "Breite der Gravur (mm)",
                "Largeur de la gravure (mm)",
                "Larghezza dell'incisione (mm)",
                "彫刻の幅 (mm)",
                "雕刻宽度 (mm)",
                "Largura da gravação (mm)",
            ],
            Msg::DotMm => [
                "Size of a dot (mm)",
                "Tamaño de un punto (mm)",
                "Größe eines Punkts (mm)",
                "Taille d'un point (mm)",
                "Dimensione di un punto (mm)",
                "ドットの大きさ (mm)",
                "点的大小 (mm)",
                "Tamanho de um ponto (mm)",
            ],
            Msg::Dither => [
                "Dithering",
                "Tramado",
                "Rasterung",
                "Tramage",
                "Retinatura",
                "ディザリング",
                "抖动",
                "Tramado",
            ],
            Msg::DitherFs => [
                "Floyd-Steinberg",
                "Floyd-Steinberg",
                "Floyd-Steinberg",
                "Floyd-Steinberg",
                "Floyd-Steinberg",
                "Floyd-Steinberg",
                "Floyd-Steinberg",
                "Floyd-Steinberg",
            ],
            Msg::DitherAtkinson => [
                "Atkinson",
                "Atkinson",
                "Atkinson",
                "Atkinson",
                "Atkinson",
                "Atkinson",
                "Atkinson",
                "Atkinson",
            ],
            Msg::DitherThreshold => [
                "Threshold",
                "Umbral",
                "Schwellwert",
                "Seuil",
                "Soglia",
                "しきい値",
                "阈值",
                "Limiar",
            ],
            Msg::Threshold => [
                "Threshold (0 to 255)",
                "Umbral (0 a 255)",
                "Schwellwert (0 bis 255)",
                "Seuil (0 à 255)",
                "Soglia (da 0 a 255)",
                "しきい値（0～255）",
                "阈值（0 到 255）",
                "Limiar (0 a 255)",
            ],
            Msg::Feed => [
                "Feed (mm/min)",
                "Avance (mm/min)",
                "Vorschub (mm/min)",
                "Avance (mm/min)",
                "Avanzamento (mm/min)",
                "送り速度 (mm/min)",
                "进给速度 (mm/min)",
                "Avanço (mm/min)",
            ],
            Msg::Power => [
                "Power (% of full power)",
                "Potencia (% de la potencia máxima)",
                "Leistung (% der vollen Leistung)",
                "Puissance (% de la pleine puissance)",
                "Potenza (% della piena potenza)",
                "出力（全出力の %）",
                "功率（满功率的百分比）",
                "Potência (% da potência máxima)",
            ],
            Msg::SMax => [
                "Full power value (S, $30 of GRBL)",
                "Valor de potencia máxima (S, $30 de GRBL)",
                "Wert der vollen Leistung (S, $30 von GRBL)",
                "Valeur de pleine puissance (S, $30 de GRBL)",
                "Valore di piena potenza (S, $30 di GRBL)",
                "全出力の値（S、GRBL の $30）",
                "满功率值（S，GRBL 的 $30）",
                "Valor da potência máxima (S, $30 do GRBL)",
            ],
            Msg::Overscan => [
                "Overscan (mm)",
                "Sobrecarrera (mm)",
                "Überlauf (mm)",
                "Dépassement (mm)",
                "Extracorsa (mm)",
                "オーバースキャン (mm)",
                "超程 (mm)",
                "Sobrecurso (mm)",
            ],
            Msg::OverscanAuto => [
                "Work the overscan out from the acceleration",
                "Calcular la sobrecarrera con la aceleración",
                "Überlauf aus der Beschleunigung berechnen",
                "Calculer le dépassement avec l'accélération",
                "Calcolare l'extracorsa con l'accelerazione",
                "加速度からオーバースキャンを計算",
                "根据加速度计算超程",
                "Calcular o sobrecurso com a aceleração",
            ],
            Msg::Accel => [
                "Acceleration (mm/s²)",
                "Aceleración (mm/s²)",
                "Beschleunigung (mm/s²)",
                "Accélération (mm/s²)",
                "Accelerazione (mm/s²)",
                "加速度 (mm/s²)",
                "加速度 (mm/s²)",
                "Aceleração (mm/s²)",
            ],
            Msg::OriginX => [
                "Origin X (mm)",
                "Origen X (mm)",
                "Ursprung X (mm)",
                "Origine X (mm)",
                "Origine X (mm)",
                "原点 X (mm)",
                "原点 X (mm)",
                "Origem X (mm)",
            ],
            Msg::OriginY => [
                "Origin Y (mm)",
                "Origen Y (mm)",
                "Ursprung Y (mm)",
                "Origine Y (mm)",
                "Origine Y (mm)",
                "原点 Y (mm)",
                "原点 Y (mm)",
                "Origem Y (mm)",
            ],
            Msg::Powers => [
                "Powers (percent, separated by commas)",
                "Potencias (porcentaje, separadas por comas)",
                "Leistungen (Prozent, durch Kommas getrennt)",
                "Puissances (pourcentage, séparées par des virgules)",
                "Potenze (percentuale, separate da virgole)",
                "出力（％、カンマ区切り）",
                "功率（百分比，用逗号分隔）",
                "Potências (percentagem, separadas por vírgulas)",
            ],
            Msg::Feeds => [
                "Feeds (mm/min, separated by commas)",
                "Avances (mm/min, separados por comas)",
                "Vorschübe (mm/min, durch Kommas getrennt)",
                "Avances (mm/min, séparés par des virgules)",
                "Avanzamenti (mm/min, separati da virgole)",
                "送り速度（mm/min、カンマ区切り）",
                "进给速度（mm/min，用逗号分隔）",
                "Avanços (mm/min, separados por vírgulas)",
            ],
            Msg::LegendBurn => [
                "Burning (the colour follows the power)",
                "Quemando (el color sigue la potencia)",
                "Brennen (die Farbe folgt der Leistung)",
                "Gravure (la couleur suit la puissance)",
                "Incisione (il colore segue la potenza)",
                "照射（色は出力に応じる）",
                "烧蚀（颜色随功率变化）",
                "A queimar (a cor segue a potência)",
            ],
            Msg::LegendTravel => [
                "Travel and overscan (laser off)",
                "Desplazamiento y sobrecarrera (láser apagado)",
                "Verfahrweg und Überlauf (Laser aus)",
                "Déplacement et dépassement (laser éteint)",
                "Spostamento ed extracorsa (laser spento)",
                "移動とオーバースキャン（レーザー消灯）",
                "移动与超程（激光关闭）",
                "Deslocamento e sobrecurso (laser desligado)",
            ],
            Msg::LegendLow => [
                "low power",
                "potencia baja",
                "geringe Leistung",
                "faible puissance",
                "bassa potenza",
                "低出力",
                "低功率",
                "potência baixa",
            ],
            Msg::LegendHigh => [
                "high power",
                "potencia alta",
                "hohe Leistung",
                "forte puissance",
                "alta potenza",
                "高出力",
                "高功率",
                "potência alta",
            ],
            Msg::DropHint => [
                "Open or drop a PNG or PGM picture here",
                "Abre o suelta aquí una imagen PNG o PGM",
                "Öffnen oder legen Sie hier ein PNG- oder PGM-Bild ab",
                "Ouvrez ou déposez ici une image PNG ou PGM",
                "Apri o trascina qui un'immagine PNG o PGM",
                "PNG または PGM 画像をここに開く、またはドロップしてください",
                "在此打开或拖入 PNG 或 PGM 图像",
                "Abra ou largue aqui uma imagem PNG ou PGM",
            ],
            Msg::NoVectors => [
                "Vector contours are not cut yet: only pictures",
                "Los contornos vectoriales aún no se cortan: solo imágenes",
                "Vektorkonturen werden noch nicht geschnitten: nur Bilder",
                "Les contours vectoriels ne sont pas encore découpés : images seulement",
                "I contorni vettoriali non si tagliano ancora: solo immagini",
                "ベクター輪郭の切断はまだありません：画像のみ",
                "尚不能切割矢量轮廓：仅支持图像",
                "Os contornos vetoriais ainda não se cortam: só imagens",
            ],
            Msg::InfoImage => [
                "{0} x {1} pixels, mean grey {2} of 255",
                "{0} x {1} píxeles, gris medio {2} de 255",
                "{0} x {1} Pixel, mittleres Grau {2} von 255",
                "{0} x {1} pixels, gris moyen {2} sur 255",
                "{0} x {1} pixel, grigio medio {2} su 255",
                "{0} x {1} ピクセル、平均グレー {2} / 255",
                "{0} x {1} 像素，平均灰度 {2} / 255",
                "{0} x {1} píxeis, cinzento médio {2} de 255",
            ],
            Msg::ImageOpened => [
                "{0}: {1} x {2} pixels",
                "{0}: {1} x {2} píxeles",
                "{0}: {1} x {2} Pixel",
                "{0} : {1} x {2} pixels",
                "{0}: {1} x {2} pixel",
                "{0}: {1} x {2} ピクセル",
                "{0}：{1} x {2} 像素",
                "{0}: {1} x {2} píxeis",
            ],
            Msg::Engraved => [
                "{0} x {1} dots ({2} x {3} mm), {4} lines, {5} runs, {6} mm burned, about {7} s",
                "{0} x {1} puntos ({2} x {3} mm), {4} líneas, {5} tramos, {6} mm quemados, unos {7} s",
                "{0} x {1} Punkte ({2} x {3} mm), {4} Zeilen, {5} Läufe, {6} mm gebrannt, etwa {7} s",
                "{0} x {1} points ({2} x {3} mm), {4} lignes, {5} passages, {6} mm gravés, environ {7} s",
                "{0} x {1} punti ({2} x {3} mm), {4} righe, {5} tratti, {6} mm incisi, circa {7} s",
                "{0} x {1} ドット（{2} x {3} mm）、{4} 行、{5} 区間、照射長 {6} mm、約 {7} 秒",
                "{0} x {1} 个点（{2} x {3} mm），{4} 行，{5} 段，烧蚀 {6} mm，约 {7} 秒",
                "{0} x {1} pontos ({2} x {3} mm), {4} linhas, {5} troços, {6} mm queimados, cerca de {7} s",
            ],
            Msg::Calibrated => [
                "{0} squares of the grid",
                "{0} cuadrados de la rejilla",
                "{0} Quadrate des Rasters",
                "{0} carrés de la grille",
                "{0} quadrati della griglia",
                "グリッドの {0} 個の四角",
                "网格的 {0} 个方块",
                "{0} quadrados da grelha",
            ],
            Msg::NeedImage => [
                "Open a PNG or PGM picture first",
                "Abre primero una imagen PNG o PGM",
                "Öffnen Sie zuerst ein PNG- oder PGM-Bild",
                "Ouvrez d'abord une image PNG ou PGM",
                "Apri prima un'immagine PNG o PGM",
                "先に PNG または PGM 画像を開いてください",
                "请先打开 PNG 或 PGM 图像",
                "Abra primeiro uma imagem PNG ou PGM",
            ],
            Msg::Truncated => [
                "The picture shows only the first {0} segments",
                "El dibujo solo muestra los primeros {0} segmentos",
                "Die Zeichnung zeigt nur die ersten {0} Segmente",
                "Le dessin ne montre que les {0} premiers segments",
                "Il disegno mostra solo i primi {0} segmenti",
                "図には最初の {0} 区間のみ表示します",
                "图中仅显示前 {0} 段",
                "O desenho mostra apenas os primeiros {0} segmentos",
            ],
        }
    }

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.words()[lang as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sentence_has_a_text_in_each_of_the_eight_languages() {
        for m in Msg::ALL {
            for l in Lang::ALL {
                assert!(!m.tr(l).trim().is_empty(), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_list_of_sentences_has_no_repeats() {
        for (i, a) in Msg::ALL.iter().enumerate() {
            assert!(!Msg::ALL[i + 1..].contains(a), "{a:?} is listed twice");
        }
    }

    #[test]
    fn a_template_has_the_same_places_in_every_language() {
        for m in Msg::ALL {
            let places = |l: Lang| {
                (0..8)
                    .filter(|k| m.tr(l).contains(&format!("{{{k}}}")))
                    .count()
            };
            for l in Lang::ALL {
                assert_eq!(places(l), places(Lang::En), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_order_of_the_languages_matches_the_lists() {
        for (k, l) in Lang::ALL.into_iter().enumerate() {
            assert_eq!(l as usize, k);
        }
        assert_eq!(Msg::Cancel.tr(Lang::Es), "Cancelar");
        assert_eq!(Msg::Cancel.tr(Lang::De), "Abbrechen");
        assert_eq!(Msg::Cancel.tr(Lang::Pt), "Cancelar");
    }

    #[test]
    fn a_language_is_found_from_the_code_of_the_system() {
        assert_eq!(Lang::from_code("es_ES.UTF-8"), Some(Lang::Es));
        assert_eq!(Lang::from_code("PT-pt"), Some(Lang::Pt));
        assert_eq!(Lang::from_code("ja"), Some(Lang::Ja));
        assert_eq!(Lang::from_code("xx"), None);
        assert_eq!(Lang::from_code(""), None);
    }

    #[test]
    fn the_places_of_a_template_are_filled() {
        assert_eq!(fill("{1} and {0}", &["a", "b"]), "b and a");
        assert_eq!(fill("none", &["a"]), "none");
    }
}
