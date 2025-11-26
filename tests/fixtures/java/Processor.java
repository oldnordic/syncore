package com.example;

/**
 * Interface for processing data.
 */
public interface Processor {
    void process();
    
    default boolean canProcess(String data) {
        return data != null && !data.isEmpty();
    }
}