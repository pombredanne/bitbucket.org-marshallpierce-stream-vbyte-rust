The file data.bin was generated as follows using the reference Stream VByte implementation (https://github.com/lemire/streamvbyte) :

```
  int N = 5000;
  uint32_t *datain = malloc(N * sizeof(uint32_t));
  uint8_t *compressedbuffer = malloc(N * sizeof(uint32_t));
  for (int k = 0; k < N; ++k)
    datain[k] = k * 100;
  size_t compsize = streamvbyte_encode(datain, N, compressedbuffer); // encoding
  const char * filename = "data.bin";
  printf("I will write the data to %s \n", filename);
  FILE *f = fopen(filename, "w");
  size_t bw = fwrite(compressedbuffer, 1, compsize, f);
  fclose(f);
```

That is, it contains 5000 integers: 0, 100, 200, ... 
