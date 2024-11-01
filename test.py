from typing import Annotated
from msgspec import Meta, Struct, field
import pysqlx


class ExampleModel(Struct):
    a: int
    b: dict[str, str]
    c: Annotated[bytes, Meta(extra={'index': True})]


db = pysqlx.SqlxDb('sqlite:////tmp/data.db')

for k, v in ExampleModel.__annotations__.items():
    print(k, dir(v))

print()
db.register_model(ExampleModel)
