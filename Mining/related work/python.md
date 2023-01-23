- **collect bug report**
  - filter by keywords in issues' title/labels
- **Mining and Normalizing Single-hunk Bugs**
  - Mining:
    - **obtain all changed files** according to bug reports's commit id; select the commits which only have **one** changed file
    - select the files that changed only **one** method
    - **Identify the code differences** in bug-finxing code changes
  - Nomalizing at AST level
    - variables
    - argumennts
    - ...
- **Cluster single-hunk bugs**
  - 粗粒度分类：transform bug codes into AST and classified them according to their **AST types**; then classify them according to **the number of bug code lines**
  - 聚类
    - calculate the AST edit distance to evaluate code similarity
    - If the edit distance of the bug code and the edit distance of their corresponding fixing code is both the same, then the hunks are the same —— candidate hunks
    - 如果bug代码和对应修复后代码的edit distance都在一个区间内，那么说他们不是the same but similar——suspicious hunks
- **Evaluation**
  - 聚类成员多于3个
  - bug-fixing code不能是来自冗余代码
- **Manual Review**
  - divide the clusters into 3 types:
    - Bug-fix
    - Fix-induced
    - Refactoring (后面经讨论后删除了)
  - 如果suspicious hunks和candidate hunks有相同语义，就可以合并进去，如果没有能够合并的suspicious hunks，就人工总结成一些formal pattern



- **RQ**
  - **What are the common fix patterns in Python?**
  - **Which fix patterns are specific to Python?**
  - **How many single-hunk bugs from BugsInPy and QuixBugs can be fixed by fix patterns?**
  - **Are the fix patterns we proposed effective in practice?
    **