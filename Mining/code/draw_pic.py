import matplotlib.pyplot as plt

# 数据
x = ['A', 'B', 'C']
y = [10, 15, 17]

# 绘制柱状图
plt.bar(x, y)

# 添加标题和标签
plt.title('Example Bar Chart')
plt.xlabel('Categories')
plt.ylabel('Values')

# 保存图像为 png 文件
plt.savefig('example.png')

# 显示图形
plt.show()
