# -*- coding: utf-8 -*-

import json

def load_script(path):
    try:
        with open(path) as fp:
            obj = json.load(fp)
        return obj
    except IOError:
        # print 'load_script(%s) IOError' % path
        return None


def save_script(fp, x):
    """
    [segment_name]
    #key_name
    :data
    ;comment
    """
    if type(x) is not dict:
        return False
    json.dump(x, fp, sort_keys=True, ensure_ascii=False, indent=4)
