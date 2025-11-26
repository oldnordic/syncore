package com.example.models;

import java.util.List;
import java.util.ArrayList;
import java.util.Map;
import java.util.HashMap;

/**
 * A more complex class with inheritance and interfaces.
 */
public class ComplexClass extends SimpleClass implements Processor {
    private List<String> data;
    private Map<String, Integer> metrics;
    
    public ComplexClass(String name) {
        super(name);
        this.data = new ArrayList<>();
        this.metrics = new HashMap<>();
    }
    
    @Override
    public void process() {
        for (String item : data) {
            metrics.put(item, item.length());
        }
    }
    
    public void addData(String item) {
        data.add(item);
    }
    
    public List<String> getData() {
        return new ArrayList<>(data);
    }
    
    public Map<String, Integer> getMetrics() {
        return new HashMap<>(metrics);
    }
    
    private void validateData(String item) {
        if (item == null || item.trim().isEmpty()) {
            throw new IllegalArgumentException("Item cannot be null or empty");
        }
    }
}