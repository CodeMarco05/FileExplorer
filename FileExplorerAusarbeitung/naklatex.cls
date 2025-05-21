\NeedsTeXFormat{LaTeX2e}
  \ProvidesClass{naklatex}

  % Basisklasse laden
    \LoadClass[11pt,a4paper]{article}

  % Pakete
  \RequirePackage[ngerman]{babel}                                             % deutsches Sprachpaket
  \RequirePackage[utf8]{inputenc}                                             % UTF8 Support
  \RequirePackage[babel, german=quotes]{csquotes}                             % Erweiterte Zitationsoptionen
  \RequirePackage{graphicx}                                                   % Ermöglicht das Einbinden von Bildern
\RequirePackage[justification=RaggedRight, singlelinecheck=false]{caption}  % Ermöglicht das Beschriften von Bildern
\RequirePackage{subcaption}                                                 % Ermöglicht eine Beschriftung für mehrere Bilder
\RequirePackage{microtype}													% Tool zur verbesserten Textausrichtung in der generierten PDF, siehe http://www.khirevich.com/latex/microtype/
\RequirePackage{eurosym}                                                    % Euro Symbol
\RequirePackage{amsmath}
\RequirePackage[titles]{tocloft}											% Packet für die Erstellung des Anhangs-Verzeichnisses
\RequirePackage[parfill]{parskip} 											% Keine Einrückung bei neuen Absätzen

% Times New Roman und Zeilenabstand
%\RequirePackage{mathptmx}
%\RequirePackage[scaled=0.9]{helvet}
%\RequirePackage{courier}
\RequirePackage[T1]{fontenc}                                                % Westeuropäisches Encoding & bessere Silbentrennung
\RequirePackage{lmodern}
\RequirePackage{setspace}
\setstretch{1.2}

% Seitenränder
\RequirePackage[left=2cm,right=2cm,top=2cm,bottom=2cm,footskip=1cm]{geometry}

% Seitenumbrüche per Section
\RequirePackage{titlesec}
\newcommand{\sectionbreak}{\cleardoublepage}

% Sections unterstreichen
\titleformat{\section}{\normalfont\Large\bfseries}{\thesection}{1em}{}[{\titlerule[0.8pt]}]

% Kopf- und Fußzeile
\RequirePackage{fancyhdr}
\pagestyle{fancy} 															% eigener Seitenstil
\fancyhf{} 																	% alle Kopf- und Fußzeilenfelder bereinigen
\renewcommand{\headrulewidth}{0pt} 											% obere Trennlinie entfernen
\fancyfoot[R]{\thepage} 													% Seitennummer nach rechts schieben

% Anhangsverzeichnis erstellen, sodass es befüllt werden kann
\newcommand{\anhangsverzeichnisName}{Anhang}
\newlistof{anhangselement}{ahv}{\anhangsverzeichnisName}
\newcommand{\anhangselement}[1]
{
    \refstepcounter{anhangselement}
    \addcontentsline{ahv}{anhangselement}
    {\protect\numberline{\theanhangselement}#1}\par
}
\newcommand{\addAnhangselement}[1]{
	\phantomsection
	\anhangselement{#1}
}

% Seitennummerierung für die einzelnen Environments vorbereiten
\newcommand{\romannumbering}{
	\pagenumbering{Roman}
	\setcounter{page}{1}}

\newcommand{\arabicnumbering}{
	\newcounter{frontmatter}
	\setcounter{frontmatter}{\value{page}}
	\pagenumbering{arabic}
	\setcounter{page}{1}}

\newcommand{\romannumberingtwo}{
	\pagenumbering{Roman}
	\setcounter{page}{\value{frontmatter}}}

\newenvironment{frontmatter}{\romannumbering}{\clearpage}
\newenvironment{mainmatter}{\arabicnumbering}{\clearpage\phantomsection}
\newenvironment{backmatter}{\romannumberingtwo}{}

% Verzeichnisbefehle ergänzen
\newcommand{\deckblatt}{
	\input{frontmatter/titlepage}
	\clearpage
	% \sectionbreak
}

\newcommand{\myabstract}{
	\input{dokumente/0_abstract.tex}
	% \sectionbreak
}

\newcommand{\inhaltsverzeichnis}{
        \restoregeometry
	\tableofcontents
	\sectionbreak
	\phantomsection
}

\newcommand{\abbildungsverzeichnis}{
	\addcontentsline{toc}{section}{Abbildungsverzeichnis}
	\addtocontents{lof}{\vskip 0.4cm}
	\listoffigures
	\sectionbreak
	\phantomsection
}

\newcommand{\tabellenverzeichnis}{
	\addcontentsline{toc}{section}{Tabellenverzeichnis}
	\addtocontents{lot}{\vskip 0.4cm}
	\listoftables
	\sectionbreak
	\phantomsection
}

\newcommand{\acronyme}{
	\printglossary[type=\acronymtype]
	\clearpage
	\phantomsection
}

\newcommand{\glossar}{
	\printglossary
	\clearpage
	\phantomsection
}

\newcommand{\literaturverzeichnis}{
	\addcontentsline{toc}{section}{Literaturverzeichnis}
	\printbibliography[title={Literaturverzeichnis}]
	\clearpage
	\phantomsection
}

\newcommand{\anhang}{
	\addtocontents{ahv}{\vskip 0.3cm}
	\listofanhangselement
	\addcontentsline{toc}{section}{Anhang}
	\clearpage
	\phantomsection	

	\input{dokumente/99_anhang.tex}
	\clearpage
	\phantomsection
}

\newcommand{\anhangsverzeichnis}{
	\listofanhangselement
	\clearpage
	\phantomsection
}

\newcommand{\eidErklaerung}{
	\input{dokumente/99_eidesstaatliche_erklaerung.tex}
    \clearpage
    \phantomsection
}

\newcommand{\Aufteilung}{
    \input{dokumente/99_Einteilung}
    \clearpage
    \phantomsection
}

% Hyperref muss am Ende, allerdings vor Glossaries geladen werden
\RequirePackage[hidelinks]{hyperref}
\hypersetup{bookmarksnumbered}

% Glossar-Paket laden und konfigurieren
\RequirePackage[automake,nonumberlist,acronym,toc]{glossaries}
\makeglossaries
\renewcommand{\glsgroupskip}{} % Zeilenabstand im Glossarverzeichis verringern
