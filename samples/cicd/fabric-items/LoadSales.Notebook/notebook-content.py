# Fabric notebook source

# METADATA ********************

# META {
# META   "kernel_info": {
# META     "name": "synapse_pyspark"
# META   },
# META   "dependencies": {
# META     "lakehouse": {
# META       "default_lakehouse": "11111111-1111-4111-8111-111111111111",
# META       "default_lakehouse_name": "SalesLakehouse",
# META       "default_lakehouse_workspace_id": "00000000-0000-0000-0000-000000000000"
# META     }
# META   }
# META }

# CELL ********************

from datetime import date

from pyspark.sql import SparkSession


def sales_rows() -> list[tuple[int, date, str, int, float]]:
    return [
        (1, date(2026, 1, 5), "Road bike", 2, 2400.0),
        (2, date(2026, 1, 7), "Helmet", 5, 375.0),
        (3, date(2026, 1, 12), "Touring bike", 1, 1650.0),
        (4, date(2026, 2, 2), "Road bike", 1, 1200.0),
    ]


spark = SparkSession.builder.getOrCreate()
sales = spark.createDataFrame(
    sales_rows(),
    schema=["order_id", "order_date", "product", "units", "amount"],
)
sales.write.format("delta").mode("overwrite").saveAsTable("sales")
