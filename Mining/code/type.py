import json
import sys
import csv
if __name__ == '__main__':
    data = [
    ['John', 'Doe', 30, 'john.doe@example.com'],
    ['Jane', 'Smith', 25, 'jane.smith@example.com'],
    ['Mike', 'Johnson', 35, 'mike.johnson@example.com']
    ]
    dd = ['sb1', 'sb2', 'sb3']
    with open('outpu.csv', 'w') as file:
        writer = csv.writer(file)
        
        for i in range(0, 3):
            writer.writerow([dd[i]] + data[i])