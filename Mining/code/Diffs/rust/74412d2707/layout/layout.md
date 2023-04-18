File_Code/rust/74412d2707/layout/layout_after.rs --- Rust
 40 "<!DOCTYPE html>\                                                                                                                                         40 "<!DOCTYPE html>\
 41 <html lang=\"en\">\                                                                                                                                       41 <html lang=\"en\">\
 42 <head>\                                                                                                                                                   42 <head>\
 43     <meta charset=\"utf-8\">\                                                                                                                             43     <meta charset=\"utf-8\">\
 44     <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\                                                                           44     <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\
 45     <meta name=\"generator\" content=\"rustdoc\">\                                                                                                        45     <meta name=\"generator\" content=\"rustdoc\">\
 46     <meta name=\"description\" content=\"{description}\">\                                                                                                46     <meta name=\"description\" content=\"{description}\">\
 47     <meta name=\"keywords\" content=\"{keywords}\">\                                                                                                      47     <meta name=\"keywords\" content=\"{keywords}\">\
 48     <title>{title}</title>\                                                                                                                               48     <title>{title}</title>\
 49     <link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}normalize{suffix}.css\">\                                                                49     <link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}normalize{suffix}.css\">\
 50     <link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}rustdoc{suffix}.css\" \                                                                  50     <link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}rustdoc{suffix}.css\" \
 51           id=\"mainThemeStyle\">\                                                                                                                         51           id=\"mainThemeStyle\">\
 52     {themes}\                                                                                                                                             52     {themes}\
 53     <link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}dark{suffix}.css\">\                                                                     53     <link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}dark{suffix}.css\">\
 54     <link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}light{suffix}.css\" \                                                                    54     <link rel=\"stylesheet\" type=\"text/css\" href=\"{root_path}light{suffix}.css\" \
 55           id=\"themeStyle\">\                                                                                                                             55           id=\"themeStyle\">\
 56     <script src=\"{root_path}storage{suffix}.js\"></script>\                                                                                              56     <script src=\"{root_path}storage{suffix}.js\"></script>\
 57     {css_extension}\                                                                                                                                      57     {css_extension}\
 58     {favicon}\                                                                                                                                            58     {favicon}\
 59     {in_header}\                                                                                                                                          59     {in_header}\
 60 </head>\                                                                                                                                                  60 </head>\
 61 <body class=\"rustdoc {css_class}\">\                                                                                                                     61 <body class=\"rustdoc {css_class}\">\
 62     <!--[if lte IE 8]>\                                                                                                                                   62     <!--[if lte IE 8]>\
 63     <div class=\"warning\">\                                                                                                                              63     <div class=\"warning\">\
 64         This old browser is unsupported and will most likely display funky \                                                                              64         This old browser is unsupported and will most likely display funky \
 65         things.\                                                                                                                                          65         things.\
 66     </div>\                                                                                                                                               66     </div>\
 67     <![endif]-->\                                                                                                                                         67     <![endif]-->\
 68     {before_content}\                                                                                                                                     68     {before_content}\
 69     <nav class=\"sidebar\">\                                                                                                                              69     <nav class=\"sidebar\">\
 70         <div class=\"sidebar-menu\">&#9776;</div>\                                                                                                        70         <div class=\"sidebar-menu\">&#9776;</div>\
 71         {logo}\                                                                                                                                           71         {logo}\
 72         {sidebar}\                                                                                                                                        72         {sidebar}\
 73     </nav>\                                                                                                                                               73     </nav>\
 74     <div class=\"theme-picker\">\                                                                                                                         74     <div class=\"theme-picker\">\
 75         <button id=\"theme-picker\" aria-label=\"Pick another theme!\">\                                                                                  75         <button id=\"theme-picker\" aria-label=\"Pick another theme!\">\
 76             <img src=\"{root_path}brush{suffix}.svg\" width=\"18\" alt=\"Pick another theme!\">\                                                          76             <img src=\"{root_path}brush{suffix}.svg\" width=\"18\" alt=\"Pick another theme!\">\
 77         </button>\                                                                                                                                        77         </button>\
 78         <div id=\"theme-choices\"></div>\                                                                                                                 78         <div id=\"theme-choices\"></div>\
 79     </div>\                                                                                                                                               79     </div>\
 80     <script src=\"{root_path}theme{suffix}.js\"></script>\                                                                                                80     <script src=\"{root_path}theme{suffix}.js\"></script>\
 81     <nav class=\"sub\">\                                                                                                                                  81     <nav class=\"sub\">\
 82         <form class=\"search-form js-only\">\                                                                                                             82         <form class=\"search-form js-only\">\
 83             <div class=\"search-container\">\                                                                                                             83             <div class=\"search-container\">\
 84                 <input class=\"search-input\" name=\"search\" \                                                                                           84                 <input class=\"search-input\" name=\"search\" \
 85                        autocomplete=\"off\" \                                                                                                             85                        autocomplete=\"off\" \
 86                        placeholder=\"Click or press ‘S’ to search, ‘?’ for more options…\" \                                                              86                        placeholder=\"Click or press ‘S’ to search, ‘?’ for more options…\" \
 87                        type=\"search\">\                                                                                                                  87                        type=\"search\">\
 88                 <a id=\"settings-menu\" href=\"{root_path}settings.html\">\                                                                               88                 <a id=\"settings-menu\" href=\"{root_path}settings.html\">\
 89                     <img src=\"{root_path}wheel{suffix}.svg\" width=\"18\" alt=\"Change settings\">\                                                      89                     <img src=\"{root_path}wheel{suffix}.svg\" width=\"18\" alt=\"Change settings\">\
 90                 </a>\                                                                                                                                     90                 </a>\
 91             </div>\                                                                                                                                       91             </div>\
 92         </form>\                                                                                                                                          92         </form>\
 93     </nav>\                                                                                                                                               93     </nav>\
 94     <section id=\"main\" class=\"content\">{content}</section>\                                                                                           94     <section id=\"main\" class=\"content\">{content}</section>\
 95     <section id=\"search\" class=\"content hidden\"></section>\                                                                                           95     <section id=\"search\" class=\"content hidden\"></section>\
 96     <section class=\"footer\"></section>\                                                                                                                 96     <section class=\"footer\"></section>\
 97     <aside id=\"help\" class=\"hidden\">\                                                                                                                 97     <aside id=\"help\" class=\"hidden\">\
 98         <div>\                                                                                                                                            98         <div>\
 99             <h1 class=\"hidden\">Help</h1>\                                                                                                               99             <h1 class=\"hidden\">Help</h1>\
100             <div class=\"shortcuts\">\                                                                                                                   100             <div class=\"shortcuts\">\
101                 <h2>Keyboard Shortcuts</h2>\                                                                                                             101                 <h2>Keyboard Shortcuts</h2>\
102                 <dl>\                                                                                                                                    102                 <dl>\
103                     <dt><kbd>?</kbd></dt>\                                                                                                               103                     <dt><kbd>?</kbd></dt>\
104                     <dd>Show this help dialog</dd>\                                                                                                      104                     <dd>Show this help dialog</dd>\
105                     <dt><kbd>S</kbd></dt>\                                                                                                               105                     <dt><kbd>S</kbd></dt>\
106                     <dd>Focus the search field</dd>\                                                                                                     106                     <dd>Focus the search field</dd>\
107                     <dt><kbd>↑</kbd></dt>\                                                                                                               107                     <dt><kbd>↑</kbd></dt>\
108                     <dd>Move up in search results</dd>\                                                                                                  108                     <dd>Move up in search results</dd>\
109                     <dt><kbd>↓</kbd></dt>\                                                                                                               109                     <dt><kbd>↓</kbd></dt>\
110                     <dd>Move down in search results</dd>\                                                                                                110                     <dd>Move down in search results</dd>\
111                     <dt><kbd>↹</kbd></dt>\                                                                                                               111                     <dt><kbd>↹</kbd></dt>\
112                     <dd>Switch tab</dd>\                                                                                                                 112                     <dd>Switch tab</dd>\
113                     <dt><kbd>&#9166;</kbd></dt>\                                                                                                         113                     <dt><kbd>&#9166;</kbd></dt>\
114                     <dd>Go to active search result</dd>\                                                                                                 114                     <dd>Go to active search result</dd>\
115                     <dt><kbd>+</kbd></dt>\                                                                                                               115                     <dt><kbd>+</kbd></dt>\
116                     <dd>Expand all sections</dd>\                                                                                                        116                     <dd>Expand all sections</dd>\
117                     <dt><kbd>-</kbd></dt>\                                                                                                               117                     <dt><kbd>-</kbd></dt>\
118                     <dd>Collapse all sections</dd>\                                                                                                      118                     <dd>Collapse all sections</dd>\
119                 </dl>\                                                                                                                                   119                 </dl>\
120             </div>\                                                                                                                                      120             </div>\
121             <div class=\"infos\">\                                                                                                                       121             <div class=\"infos\">\
122                 <h2>Search Tricks</h2>\                                                                                                                  122                 <h2>Search Tricks</h2>\
123                 <p>\                                                                                                                                     123                 <p>\
124                     Prefix searches with a type followed by a colon (e.g. \                                                                              124                     Prefix searches with a type followed by a colon (e.g. \
125                     <code>fn:</code>) to restrict the search to a given type.\                                                                           125                     <code>fn:</code>) to restrict the search to a given type.\
126                 </p>\                                                                                                                                    126                 </p>\
127                 <p>\                                                                                                                                     127                 <p>\
128                     Accepted types are: <code>fn</code>, <code>mod</code>, \                                                                             128                     Accepted types are: <code>fn</code>, <code>mod</code>, \
129                     <code>struct</code>, <code>enum</code>, \                                                                                            129                     <code>struct</code>, <code>enum</code>, \
130                     <code>trait</code>, <code>type</code>, <code>macro</code>, \                                                                         130                     <code>trait</code>, <code>type</code>, <code>macro</code>, \
131                     and <code>const</code>.\                                                                                                             131                     and <code>const</code>.\
132                 </p>\                                                                                                                                    132                 </p>\
133                 <p>\                                                                                                                                     133                 <p>\
134                     Search functions by type signature (e.g. \                                                                                           134                     Search functions by type signature (e.g. \
135                     <code>vec -> usize</code> or <code>* -> vec</code>)\                                                                                 135                     <code>vec -> usize</code> or <code>* -> vec</code>)\
136                 </p>\                                                                                                                                    136                 </p>\
137                 <p>\                                                                                                                                     137                 <p>\
138                     Search multiple things at once by splitting your query with comma (e.g. \                                                            138                     Search multiple things at once by splitting your query with comma (e.g. \
139                     <code>str,u8</code> or <code>String,struct:Vec,test</code>)\                                                                         139                     <code>str,u8</code> or <code>String,struct:Vec,test</code>)\
140                 </p>\                                                                                                                                    140                 </p>\
141             </div>\                                                                                                                                      141             </div>\
142         </div>\                                                                                                                                          142         </div>\
143     </aside>\                                                                                                                                            143     </aside>\
144     {after_content}\                                                                                                                                     144     {after_content}\
145     <script>\                                                                                                                                            145     <script>\
146         window.rootPath = \"{root_path}\";\                                                                                                              146         window.rootPath = \"{root_path}\";\
147         window.currentCrate = \"{krate}\";\                                                                                                              147         window.currentCrate = \"{krate}\";\
148     </script>\                                                                                                                                           148     </script>\
149     <script src=\"{root_path}main{suffix}.js\"></script>\                                                                                                149     <script src=\"{root_path}aliases.js\"></script>\
150     <script defer src=\"{root_path}search-index.js\"></script>\                                                                                          150     <script src=\"{root_path}main{suffix}.js\"></script>\
151     <script defer src=\"{root_path}aliases.js\"></script>\                                                                                               151     <script defer src=\"{root_path}search-index.js\"></script>\
152 </body>\                                                                                                                                                 152 </body>\
153 </html>",                                                                                                                                                153 </html>",

