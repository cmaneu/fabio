# Fabric CI/CD sample

This sample accompanies the [CI/CD tutorial](https://ismaelmejia.com/fabio/cicd-tutorial/).
It deploys a small sales solution with:

- `SalesLakehouse` — a schema-less Lakehouse.
- `LoadSales` — a PySpark notebook that creates the `sales` Delta table.
- `SalesModel` — a Direct Lake semantic model over the table.
- `SalesReport` — a Power BI report bound to the semantic model.

The included workflow validates and lints every pull request, creates an isolated
preview workspace on a development capacity, deploys the solution, comments its
Fabric URL on the PR, and deletes the workspace when the PR closes. A push to
`main` deploys the same source to a protected production workspace.

## Use the sample

Copy this directory's contents to the root of a new repository, then follow the
tutorial to configure GitHub environments, workload identity federation, and
Fabric permissions.

Run the local checks before pushing:

```bash
python -m pip install -r requirements-dev.txt
ruff check fabric-items tests
python -m unittest discover -s tests
fabio deploy validate --source ./fabric-items \
  --parameters ./parameters.json --env preview
fabio report validate --source ./fabric-items/SalesReport.Report
```

The workflow uses the default per-item deployment strategy so Fabio can resolve
the Lakehouse, semantic model, and report logical references in dependency order.
