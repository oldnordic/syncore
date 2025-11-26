package com.example;

import java.util.List;
import java.util.ArrayList;

/**
 * A simple class for testing Java plugin functionality.
 */
public class SimpleClass {
    private String name;
    private int count;
    
    public SimpleClass(String name) {
        this.name = name;
        this.count = 0;
    }
    
    public void increment() {
        this.count++;
    }
    
    public int getCount() {
        return this.count;
    }
    
    public String getName() {
        return this.name;
    }
    
    public void processItems(List<String> items) {
        for (String item : items) {
            System.out.println("Processing: " + item);
        }
    }
}