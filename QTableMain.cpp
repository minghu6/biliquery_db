#include <iostream>
#include <algorithm>
#include <unordered_map>
#include <stdint.h>
#include <string.h>
#include <sys/socket.h>
#include <netdb.h>
#include <sys/time.h>
#include <errno.h>
#include <pthread.h>
#include <stdlib.h>
#include <unistd.h>
#include <stdio.h>
#include <ev++.h>

#include "TCPServer.hpp"
#include "Logger.h"
#include "Shared.h"
#include "utils.h"

using namespace std;
CREATE_LOGGER;

int main() {
    INIT_LOGGER;
    if (file_exists("data/biliquery.bin")) goto Loop;

{
    LOG(WARNING) << "Database not found, recreating...";
    // Load all data to map
    DirectMap temp_map;
    FILE *file = fopen("data/table", "rb");
    char buffer[4096];
    uint32_t keyseq = 0;
    while(fread(buffer, 4096, 1, file)){
        for (int i = 0; i < 1024; i++){
            keyseq++;
            uint32_t data = *(uint32_t *)(buffer+(i*4));
            temp_map.insert(std::make_pair(data, keyseq));
        }
        if (keyseq % 100 == 0) {
            printf("Please wait... %lu\r",keyseq);
            fflush(stdout);
        }

    }
    LOG(INFO) << "Import completed: " << keyseq << " items";
    fclose(file);
    FILE *db = fopen("data/biliquery.bin", "wb");
    FILE *dups = fopen("data/duplicate.bin", "wb");

    uint32_t uint32_min = 0x00000000;
    uint32_t uint32_max = 0xFFFFFFFF;

    for(uint32_t i = 0;i < uint32_max;i++){
        auto count = temp_map.count(i);
        if(!count){
            fwrite(&uint32_min,4,1,db);
            continue;
        }else if(count > 1){
            fwrite(&uint32_max,4,1,db);
        }
        auto its = temp_map.equal_range(i);
        for (auto it = its.first; it != its.second; ++it) {
            if(count == 1){
                // Write directly to file
                fwrite(&it->second,4,1,db);
            }else{
                // Duplicated item
                fwrite(&it->first,4,1,dups);
                fwrite(&it->second,4,1,dups);
            }
        }
        if(i % 100000 == 0){
            printf("Please wait... %u/%u\r",i,uint32_max);
            fflush(stdout);
        }
    }
    LOG(INFO) << "Database created";
    fclose(db);
    fclose(dups);
}

Loop:
#ifdef __APPLE__
    setenv("LIBEV_FLAGS", "8", 1);
#endif
    ev::default_loop loop;
    TCPServer serv;
    loop.run();
    return 0;
}
