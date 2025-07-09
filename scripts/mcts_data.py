import pandas as pd
import matplotlib.pyplot as plt

df = pd.read_csv("0.csv", names=["state", "win", "total"])
print(df)

df = df.groupby("state").sum().reset_index()
df["rate"] = df["win"] / df["total"]
print(df)

df = df.drop(df[df["rate"] == 0.0].sample(frac=0.875).index)
df = df.drop(df[df["rate"] == 1.0].sample(frac=0.875).index)
print(df)
hist = df["rate"].hist(bins=1000)
plt.show()

df.loc[:, ["state", "rate"]].to_csv("0_processed.csv", header=False, index=False)
