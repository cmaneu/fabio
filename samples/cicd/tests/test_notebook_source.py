import ast
import unittest
from pathlib import Path


NOTEBOOK = (
    Path(__file__).parents[1]
    / "fabric-items"
    / "LoadSales.Notebook"
    / "notebook-content.py"
)


class NotebookSourceTests(unittest.TestCase):
    def test_notebook_is_valid_python(self) -> None:
        ast.parse(NOTEBOOK.read_text(encoding="utf-8"), filename=str(NOTEBOOK))

    def test_notebook_writes_the_sales_table(self) -> None:
        source = NOTEBOOK.read_text(encoding="utf-8")
        self.assertIn('.saveAsTable("sales")', source)
        self.assertIn('"amount"', source)


if __name__ == "__main__":
    unittest.main()
