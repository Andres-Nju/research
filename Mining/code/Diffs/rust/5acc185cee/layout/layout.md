File_Code/rust/5acc185cee/layout/layout_after.rs --- Rust
 38 r##"<!DOCTYPE html>                                                                                                                                       38 r##"<!DOCTYPE html>
 39 <html lang="en">                                                                                                                                          39 <html lang="en">
 40 <head>                                                                                                                                                    40 <head>
 41     <meta charset="utf-8">                                                                                                                                41     <meta charset="utf-8">
 42     <meta name="viewport" content="width=device-width, initial-scale=1.0">                                                                                42     <meta name="viewport" content="width=device-width, initial-scale=1.0">
 43     <meta name="generator" content="rustdoc">                                                                                                             43     <meta name="generator" content="rustdoc">
 44     <meta name="description" content="{description}">                                                                                                     44     <meta name="description" content="{description}">
 45     <meta name="keywords" content="{keywords}">                                                                                                           45     <meta name="keywords" content="{keywords}">
 46                                                                                                                                                           46 
 47     <title>{title}</title>                                                                                                                                47     <title>{title}</title>
 48                                                                                                                                                           48 
 49     <link rel="stylesheet" type="text/css" href="{root_path}normalize.css">                                                                               49     <link rel="stylesheet" type="text/css" href="{root_path}normalize.css">
 50     <link rel="stylesheet" type="text/css" href="{root_path}rustdoc.css">                                                                                 50     <link rel="stylesheet" type="text/css" href="{root_path}rustdoc.css">
 51     <link rel="stylesheet" type="text/css" href="{root_path}main.css">                                                                                    51     <link rel="stylesheet" type="text/css" href="{root_path}main.css">
 52     {css_extension}                                                                                                                                       52     {css_extension}
 53                                                                                                                                                           53 
 54     {favicon}                                                                                                                                             54     {favicon}
 55     {in_header}                                                                                                                                           55     {in_header}
 56 </head>                                                                                                                                                   56 </head>
 57 <body class="rustdoc {css_class}">                                                                                                                        57 <body class="rustdoc {css_class}">
 58     <!--[if lte IE 8]>                                                                                                                                    58     <!--[if lte IE 8]>
 59     <div class="warning">                                                                                                                                 59     <div class="warning">
 60         This old browser is unsupported and will most likely display funky                                                                                60         This old browser is unsupported and will most likely display funky
 61         things.                                                                                                                                           61         things.
 62     </div>                                                                                                                                                62     </div>
 63     <![endif]-->                                                                                                                                          63     <![endif]-->
 64                                                                                                                                                           64 
 65     {before_content}                                                                                                                                      65     {before_content}
 66                                                                                                                                                           66 
 67     <nav class="sidebar">                                                                                                                                 67     <nav class="sidebar">
 68         {logo}                                                                                                                                            68         {logo}
 69         {sidebar}                                                                                                                                         69         {sidebar}
 70     </nav>                                                                                                                                                70     </nav>
 71                                                                                                                                                           71 
 72     <nav class="sub">                                                                                                                                     72     <nav class="sub">
 73         <form class="search-form js-only">                                                                                                                73         <form class="search-form js-only">
 74             <div class="search-container">                                                                                                                74             <div class="search-container">
 75                 <input class="search-input" name="search"                                                                                                 75                 <input class="search-input" name="search"
 76                        autocomplete="off"                                                                                                                 76                        autocomplete="off"
 77                        placeholder="Click or press ‘S’ to search, ‘?’ for more options…"                                                                  77                        placeholder="Click or press ‘S’ to search, ‘?’ for more options…"
 78                        type="search">                                                                                                                     78                        type="search">
 79             </div>                                                                                                                                        79             </div>
 80         </form>                                                                                                                                           80         </form>
 81     </nav>                                                                                                                                                81     </nav>
 82                                                                                                                                                           82 
 83     <section id='main' class="content">{content}</section>                                                                                                83     <section id='main' class="content">{content}</section>
 84     <section id='search' class="content hidden"></section>                                                                                                84     <section id='search' class="content hidden"></section>
 85                                                                                                                                                           85 
 86     <section class="footer"></section>                                                                                                                    86     <section class="footer"></section>
 87                                                                                                                                                           87 
 88     <aside id="help" class="hidden">                                                                                                                      88     <aside id="help" class="hidden">
 89         <div>                                                                                                                                             89         <div>
 90             <h1 class="hidden">Help</h1>                                                                                                                  90             <h1 class="hidden">Help</h1>
 91                                                                                                                                                           91 
 92             <div class="shortcuts">                                                                                                                       92             <div class="shortcuts">
 93                 <h2>Keyboard Shortcuts</h2>                                                                                                               93                 <h2>Keyboard Shortcuts</h2>
 94                                                                                                                                                           94 
 95                 <dl>                                                                                                                                      95                 <dl>
 96                     <dt>?</dt>                                                                                                                            96                     <dt>?</dt>
 97                     <dd>Show this help dialog</dd>                                                                                                        97                     <dd>Show this help dialog</dd>
 98                     <dt>S</dt>                                                                                                                            98                     <dt>S</dt>
 99                     <dd>Focus the search field</dd>                                                                                                       99                     <dd>Focus the search field</dd>
100                     <dt>&larrb;</dt>                                                                                                                     100                     <dt>↑</dt>
101                     <dd>Move up in search results</dd>                                                                                                   101                     <dd>Move up in search results</dd>
102                     <dt>&rarrb;</dt>                                                                                                                     102                     <dt>↓</dt>
103                     <dd>Move down in search results</dd>                                                                                                 103                     <dd>Move down in search results</dd>
104                     <dt>&#9166;</dt>                                                                                                                     104                     <dt>&#9166;</dt>
105                     <dd>Go to active search result</dd>                                                                                                  105                     <dd>Go to active search result</dd>
106                     <dt>+</dt>                                                                                                                           106                     <dt>+</dt>
107                     <dd>Collapse/expand all sections</dd>                                                                                                107                     <dd>Collapse/expand all sections</dd>
108                 </dl>                                                                                                                                    108                 </dl>
109             </div>                                                                                                                                       109             </div>
110                                                                                                                                                          110 
111             <div class="infos">                                                                                                                          111             <div class="infos">
112                 <h2>Search Tricks</h2>                                                                                                                   112                 <h2>Search Tricks</h2>
113                                                                                                                                                          113 
114                 <p>                                                                                                                                      114                 <p>
115                     Prefix searches with a type followed by a colon (e.g.                                                                                115                     Prefix searches with a type followed by a colon (e.g.
116                     <code>fn:</code>) to restrict the search to a given type.                                                                            116                     <code>fn:</code>) to restrict the search to a given type.
117                 </p>                                                                                                                                     117                 </p>
118                                                                                                                                                          118 
119                 <p>                                                                                                                                      119                 <p>
120                     Accepted types are: <code>fn</code>, <code>mod</code>,                                                                               120                     Accepted types are: <code>fn</code>, <code>mod</code>,
121                     <code>struct</code>, <code>enum</code>,                                                                                              121                     <code>struct</code>, <code>enum</code>,
122                     <code>trait</code>, <code>type</code>, <code>macro</code>,                                                                           122                     <code>trait</code>, <code>type</code>, <code>macro</code>,
123                     and <code>const</code>.                                                                                                              123                     and <code>const</code>.
124                 </p>                                                                                                                                     124                 </p>
125                                                                                                                                                          125 
126                 <p>                                                                                                                                      126                 <p>
127                     Search functions by type signature (e.g.                                                                                             127                     Search functions by type signature (e.g.
128                     <code>vec -> usize</code> or <code>* -> vec</code>)                                                                                  128                     <code>vec -> usize</code> or <code>* -> vec</code>)
129                 </p>                                                                                                                                     129                 </p>
130             </div>                                                                                                                                       130             </div>
131         </div>                                                                                                                                           131         </div>
132     </aside>                                                                                                                                             132     </aside>
133                                                                                                                                                          133 
134     {after_content}                                                                                                                                      134     {after_content}
135                                                                                                                                                          135 
136     <script>                                                                                                                                             136     <script>
137         window.rootPath = "{root_path}";                                                                                                                 137         window.rootPath = "{root_path}";
138         window.currentCrate = "{krate}";                                                                                                                 138         window.currentCrate = "{krate}";
139     </script>                                                                                                                                            139     </script>
140     <script src="{root_path}main.js"></script>                                                                                                           140     <script src="{root_path}main.js"></script>
141     <script defer src="{root_path}search-index.js"></script>                                                                                             141     <script defer src="{root_path}search-index.js"></script>
142 </body>                                                                                                                                                  142 </body>
143 </html>"##,                                                                                                                                              143 </html>"##,

