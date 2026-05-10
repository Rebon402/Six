from setuptools import setup

setup(
    name="six-language",
    version="0.1.0",
    py_modules=["six_cli_wrapper"],
    entry_points={
        "console_scripts": [
            "six = six_cli_wrapper:main",
        ],
    },
)
